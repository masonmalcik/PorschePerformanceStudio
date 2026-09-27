import { randomUUID } from "node:crypto";
import { connect, type AmqpConnectionManager, type ChannelWrapper } from "amqp-connection-manager";
import type { ConfirmChannel, ConsumeMessage } from "amqplib";
import { envelopeSchema, eventSchemas, type EventEnvelope, type EventPayloads, type RoutingKey } from "./contracts.js";
import { EVENTS_EXCHANGE, RETRY_EXCHANGE, assertBaseTopology, assertSubscriptionTopology } from "./topology.js";

export interface RabbitMQLogger {
  info(context: object, message: string): void;
  warn(context: object, message: string): void;
  error(context: object, message: string): void;
}

export interface RabbitMQOptions {
  urls: string[];
  prefetch?: number;
  maxRetries?: number;
  retryDelayMs?: number;
  heartbeatIntervalSeconds?: number;
  reconnectTimeSeconds?: number;
  logger?: RabbitMQLogger;
  connectionFactory?: typeof connect;
}

export class NonRetryableMessageError extends Error {
  public constructor(message: string) {
    super(message);
    this.name = "NonRetryableMessageError";
  }
}

export class RabbitMQService {
  private readonly connection: AmqpConnectionManager;
  private readonly publisher: ChannelWrapper;
  private readonly consumers = new Set<ChannelWrapper>();
  private readonly prefetch: number;
  private readonly maxRetries: number;
  private readonly retryDelayMs: number;
  private readonly logger: RabbitMQLogger;

  public constructor(options: RabbitMQOptions) {
    if (options.urls.length === 0) throw new Error("At least one RabbitMQ URL is required");
    this.prefetch = options.prefetch ?? 20;
    this.maxRetries = options.maxRetries ?? 3;
    this.retryDelayMs = options.retryDelayMs ?? 5000;
    this.logger = options.logger ?? console;
    const factory = options.connectionFactory ?? connect;
    this.connection = factory(options.urls, {
      heartbeatIntervalInSeconds: options.heartbeatIntervalSeconds ?? 5,
      reconnectTimeInSeconds: options.reconnectTimeSeconds ?? 5,
    });
    this.connection.on("connect", () => this.logger.info({}, "RabbitMQ connected"));
    this.connection.on("disconnect", (context) => this.logger.warn({ error: context.err }, "RabbitMQ disconnected; reconnecting"));
    this.publisher = this.connection.createChannel({
      name: "ecom-confirm-publisher",
      confirm: true,
      setup: async (channel: ConfirmChannel) => assertBaseTopology(channel),
    });
  }

  public async publish<K extends RoutingKey>(routingKey: K, payload: EventPayloads[K], correlationId?: string): Promise<EventEnvelope<K>> {
    const validPayload = eventSchemas[routingKey].parse(payload) as EventPayloads[K];
    const envelope: EventEnvelope<K> = {
      eventId: randomUUID(),
      type: routingKey,
      version: 1,
      occurredAt: new Date().toISOString(),
      ...(correlationId ? { correlationId } : {}),
      payload: validPayload,
    };
    await this.publisher.publish(EVENTS_EXCHANGE, routingKey, Buffer.from(JSON.stringify(envelope)), {
      persistent: true,
      contentType: "application/json",
      messageId: envelope.eventId,
      timestamp: Date.now(),
      ...(correlationId ? { correlationId } : {}),
    });
    return envelope;
  }

  public async subscribe<K extends RoutingKey>(
    queueName: string,
    routingKey: K,
    handler: (event: EventEnvelope<K>) => Promise<void>,
  ): Promise<() => Promise<void>> {
    const wrapper = this.connection.createChannel({
      name: `consumer-${queueName}`,
      confirm: true,
      setup: async (channel: ConfirmChannel) => {
        await assertSubscriptionTopology(channel, { queueName, routingKeys: [routingKey] }, this.retryDelayMs);
        await channel.prefetch(this.prefetch);
        await channel.consume(queueName, (message) => {
          if (message) void this.processMessage(channel, message, queueName, routingKey, handler);
        }, { noAck: false });
      },
    });
    this.consumers.add(wrapper);
    await wrapper.waitForConnect();
    return async () => {
      this.consumers.delete(wrapper);
      await wrapper.close();
    };
  }

  public async close(): Promise<void> {
    await Promise.all([...this.consumers].map((consumer) => consumer.close()));
    await this.publisher.close();
    await this.connection.close();
  }

  private async processMessage<K extends RoutingKey>(
    channel: ConfirmChannel,
    message: ConsumeMessage,
    queueName: string,
    routingKey: K,
    handler: (event: EventEnvelope<K>) => Promise<void>,
  ): Promise<void> {
    let envelope: EventEnvelope<K>;
    try {
      const base = envelopeSchema.parse(JSON.parse(message.content.toString("utf8")));
      if (base.type !== routingKey) throw new NonRetryableMessageError(`Expected ${routingKey}, received ${base.type}`);
      const payload = eventSchemas[routingKey].parse(base.payload) as EventPayloads[K];
      envelope = { ...base, type: routingKey, payload } as EventEnvelope<K>;
    } catch (error) {
      this.logger.warn({ error, queueName }, "Rejecting invalid event to DLQ");
      channel.nack(message, false, false);
      return;
    }

    try {
      await handler(envelope);
      channel.ack(message);
    } catch (error) {
      const retryCount = this.retryCount(message);
      if (error instanceof NonRetryableMessageError || retryCount >= this.maxRetries) {
        this.logger.error({ error, eventId: envelope.eventId, retryCount }, "Event exhausted retries; routing to DLQ");
        channel.nack(message, false, false);
        return;
      }
      try {
        await this.publisher.publish(RETRY_EXCHANGE, queueName, message.content, {
          persistent: true,
          contentType: message.properties.contentType ?? "application/json",
          messageId: message.properties.messageId,
          correlationId: message.properties.correlationId,
          headers: { ...message.properties.headers, "x-retry-count": retryCount + 1 },
        });
        channel.ack(message);
      } catch (publishError) {
        this.logger.warn({ error: publishError, eventId: envelope.eventId }, "Retry publication failed; requeueing original event");
        channel.nack(message, false, true);
      }
    }
  }

  private retryCount(message: ConsumeMessage): number {
    const value = message.properties.headers?.["x-retry-count"];
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 ? value : 0;
  }
}
