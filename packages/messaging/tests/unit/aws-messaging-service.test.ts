import { PublishCommand } from "@aws-sdk/client-sns";
import type { SQSEvent } from "aws-lambda";
import { describe, expect, it, vi } from "vitest";
import { AwsMessagingService, createSqsBatchHandler } from "../../src/aws-messaging-service.js";

describe("AwsMessagingService", () => {
  it("publishes a validated envelope with SNS filter attributes", async () => {
    const send = vi.fn().mockResolvedValue({ MessageId: "message-1" });
    const service = new AwsMessagingService({
      topicArn: "arn:aws:sns:us-east-1:123456789012:pps-events-dev",
      client: { send },
    });
    const envelope = await service.publish("payment.succeeded", {
      paymentIntentId: "pi_test_123",
      orderId: "83bbd3a8-c34d-4da8-a0bc-6fb46de87734",
      amount: 2500,
    }, "checkout-123");

    expect(envelope.type).toBe("payment.succeeded");
    expect(send).toHaveBeenCalledOnce();
    const command = send.mock.calls[0]?.[0];
    expect(command).toBeInstanceOf(PublishCommand);
    expect(command.input.MessageAttributes?.eventType?.StringValue).toBe("payment.succeeded");
  });

  it("reports only failed SQS records for retry", async () => {
    const handler = vi.fn().mockResolvedValueOnce(undefined).mockRejectedValueOnce(new Error("temporary"));
    const consume = createSqsBatchHandler("payment.failed", handler, { warn: vi.fn() });
    const envelope = (eventId: string) => JSON.stringify({
      eventId,
      type: "payment.failed",
      version: 1,
      occurredAt: "2026-09-27T00:00:00.000Z",
      payload: {
        paymentIntentId: "pi_test_failed",
        orderId: "83bbd3a8-c34d-4da8-a0bc-6fb46de87734",
        amount: 2500,
        reason: "declined",
      },
    });
    const event = {
      Records: [
        { messageId: "one", body: envelope("a4dd6c7a-69f4-4704-bc52-d999b1ed6090") },
        { messageId: "two", body: envelope("81c273e3-81f0-45f2-a548-c70b5a44390f") },
      ],
    } as SQSEvent;

    await expect(consume(event)).resolves.toEqual({ batchItemFailures: [{ itemIdentifier: "two" }] });
    expect(handler).toHaveBeenCalledTimes(2);
  });
});
