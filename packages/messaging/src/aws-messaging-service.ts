import { randomUUID } from "node:crypto";
import { PublishCommand, SNSClient } from "@aws-sdk/client-sns";
import type { SQSEvent, SQSBatchResponse, SQSRecord } from "aws-lambda";
import { envelopeSchema, eventSchemas, type EventEnvelope, type EventPayloads, type RoutingKey } from "./contracts.js";

interface SnsPublisher {
  send(command: PublishCommand): Promise<unknown>;
}

export interface AwsMessagingOptions {
  topicArn: string;
  client?: SnsPublisher;
}

export class AwsMessagingService {
  private readonly client: SnsPublisher;

  public constructor(private readonly options: AwsMessagingOptions) {
    if (!options.topicArn.startsWith("arn:aws:sns:")) throw new Error("A valid SNS topic ARN is required");
    this.client = options.client ?? new SNSClient({});
  }

  public async publish<K extends RoutingKey>(
    routingKey: K,
    payload: EventPayloads[K],
    correlationId?: string,
  ): Promise<EventEnvelope<K>> {
    const envelope: EventEnvelope<K> = {
      eventId: randomUUID(),
      type: routingKey,
      version: 1,
      occurredAt: new Date().toISOString(),
      ...(correlationId ? { correlationId } : {}),
      payload: eventSchemas[routingKey].parse(payload) as EventPayloads[K],
    };
    await this.client.send(new PublishCommand({
      TopicArn: this.options.topicArn,
      Message: JSON.stringify(envelope),
      MessageAttributes: {
        eventType: { DataType: "String", StringValue: routingKey },
        eventVersion: { DataType: "Number", StringValue: "1" },
        ...(correlationId
          ? { correlationId: { DataType: "String", StringValue: correlationId } }
          : {}),
      },
    }));
    return envelope;
  }
}

export type SqsEventHandler<K extends RoutingKey> = (event: EventEnvelope<K>) => Promise<void>;

export function createSqsBatchHandler<K extends RoutingKey>(
  routingKey: K,
  handler: SqsEventHandler<K>,
  logger: Pick<Console, "warn"> = console,
): (event: SQSEvent) => Promise<SQSBatchResponse> {
  return async (event) => {
    const batchItemFailures: SQSBatchResponse["batchItemFailures"] = [];
    for (const record of event.Records) {
      try {
        await handler(parseRecord(record, routingKey));
      } catch (error) {
        logger.warn("SQS event processing failed", { error, messageId: record.messageId, routingKey });
        batchItemFailures.push({ itemIdentifier: record.messageId });
      }
    }
    return { batchItemFailures };
  };
}

function parseRecord<K extends RoutingKey>(record: SQSRecord, routingKey: K): EventEnvelope<K> {
  const decoded = JSON.parse(record.body) as unknown;
  const message = isSnsNotification(decoded) ? JSON.parse(decoded.Message) as unknown : decoded;
  const base = envelopeSchema.parse(message);
  if (base.type !== routingKey) throw new Error(`Expected ${routingKey}, received ${base.type}`);
  const payload = eventSchemas[routingKey].parse(base.payload) as EventPayloads[K];
  return { ...base, type: routingKey, payload } as EventEnvelope<K>;
}

function isSnsNotification(value: unknown): value is { Type: "Notification"; Message: string } {
  return typeof value === "object" && value !== null
    && "Type" in value && value.Type === "Notification"
    && "Message" in value && typeof value.Message === "string";
}
