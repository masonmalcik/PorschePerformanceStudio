import type { ConfirmChannel, ConsumeMessage } from "amqplib";
import { describe, expect, it, vi } from "vitest";
import { NonRetryableMessageError, RabbitMQService } from "../../src/rabbitmq-service.js";
import { EVENTS_EXCHANGE, RETRY_EXCHANGE } from "../../src/topology.js";

const payload = {
  orderId: "11111111-1111-4111-8111-111111111111",
  userId: "user-1",
  totalAmount: "20.00",
  items: [{ productId: "product-1", quantity: 2, pricePerUnit: "10.00" }],
};

function harness() {
  let consume: ((message: ConsumeMessage | null) => void) | undefined;
  const channel = {
    assertExchange: vi.fn().mockResolvedValue({}), assertQueue: vi.fn().mockResolvedValue({}),
    bindQueue: vi.fn().mockResolvedValue({}), prefetch: vi.fn().mockResolvedValue(undefined),
    consume: vi.fn(async (_queue, callback) => { consume = callback; return { consumerTag: "tag" }; }),
    ack: vi.fn(), nack: vi.fn(),
  } as unknown as ConfirmChannel;
  const publish = vi.fn().mockResolvedValue(true);
  const publisher = { publish, close: vi.fn().mockResolvedValue(undefined) };
  const consumerWrappers: Array<{ close: ReturnType<typeof vi.fn> }> = [];
  let calls = 0;
  const connection = {
    on: vi.fn(),
    createChannel: vi.fn((options: { setup?: (value: ConfirmChannel) => Promise<void> }) => {
      calls++;
      if (calls === 1) return publisher;
      const wrapper = {
        close: vi.fn().mockResolvedValue(undefined),
        waitForConnect: vi.fn(async () => options.setup?.(channel)),
      };
      consumerWrappers.push(wrapper);
      return wrapper;
    }),
    close: vi.fn().mockResolvedValue(undefined),
  };
  const factory = vi.fn(() => connection);
  const service = new RabbitMQService({ urls: ["amqp://test"], connectionFactory: factory as never, retryDelayMs: 1, maxRetries: 3 });
  return { service, channel, publish, getConsumer: () => consume, connection, consumerWrappers };
}

function message(retryCount = 0, valid = true): ConsumeMessage {
  const envelope = valid ? {
    eventId: "22222222-2222-4222-8222-222222222222", type: "order.created", version: 1,
    occurredAt: new Date().toISOString(), payload,
  } : { invalid: true };
  return {
    content: Buffer.from(JSON.stringify(envelope)),
    fields: {} as ConsumeMessage["fields"],
    properties: {
      contentType: "application/json", contentEncoding: undefined, headers: { "x-retry-count": retryCount },
      deliveryMode: undefined, priority: undefined, correlationId: undefined, replyTo: undefined,
      expiration: undefined, messageId: "event-1", timestamp: undefined, type: undefined,
      userId: undefined, appId: undefined, clusterId: undefined,
    },
  };
}

describe("RabbitMQService", () => {
  it("publishes persistent messages and awaits publisher confirmation", async () => {
    const { service, publish } = harness();
    const event = await service.publish("order.created", payload);
    expect(event.type).toBe("order.created");
    expect(publish).toHaveBeenCalledWith(EVENTS_EXCHANGE, "order.created", expect.any(Buffer), expect.objectContaining({ persistent: true }));
  });

  it("uses manual acknowledgment only after successful processing", async () => {
    const { service, channel, getConsumer } = harness();
    const handler = vi.fn().mockResolvedValue(undefined);
    await service.subscribe("ecom.payment.order-created", "order.created", handler);
    getConsumer()?.(message());
    await vi.waitFor(() => expect(handler).toHaveBeenCalledOnce());
    expect(channel.ack).toHaveBeenCalledOnce();
    expect(channel.nack).not.toHaveBeenCalled();
    expect(channel.consume).toHaveBeenCalledWith("ecom.payment.order-created", expect.any(Function), { noAck: false });
  });

  it("confirms a bounded retry before acknowledging the original", async () => {
    const { service, channel, publish, getConsumer } = harness();
    await service.subscribe("ecom.payment.order-created", "order.created", async () => { throw new Error("temporary"); });
    getConsumer()?.(message(0));
    await vi.waitFor(() => expect(publish).toHaveBeenCalledWith(RETRY_EXCHANGE, "ecom.payment.order-created", expect.any(Buffer), expect.any(Object)));
    expect(channel.ack).toHaveBeenCalledOnce();
  });

  it("dead-letters malformed, non-retryable, and exhausted events", async () => {
    const malformed = harness();
    await malformed.service.subscribe("ecom.payment.order-created", "order.created", vi.fn());
    malformed.getConsumer()?.(message(0, false));
    await vi.waitFor(() => expect(malformed.channel.nack).toHaveBeenCalledWith(expect.anything(), false, false));

    const exhausted = harness();
    await exhausted.service.subscribe("ecom.payment.order-created", "order.created", async () => { throw new NonRetryableMessageError("bad state"); });
    exhausted.getConsumer()?.(message(3));
    await vi.waitFor(() => expect(exhausted.channel.nack).toHaveBeenCalledWith(expect.anything(), false, false));
  });
});
