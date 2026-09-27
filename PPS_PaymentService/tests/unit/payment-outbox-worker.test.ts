import { describe, expect, it, vi } from "vitest";
import { PaymentOutboxWorker } from "../../src/messaging/payment-outbox-worker.js";

describe("PaymentOutboxWorker", () => {
  it("publishes and marks a successful payment event", async () => {
    const store = {
      claimBatch: vi.fn().mockResolvedValue([{
        eventId: "event-1",
        aggregateId: "transaction-1",
        type: "PaymentProcessed",
        occurredAt: "2026-09-27T00:00:00.000Z",
        data: { stripePaymentIntentId: "pi_test_123", orderId: "order-1", amount: 4250 },
      }]),
      markPublished: vi.fn().mockResolvedValue(undefined),
    };
    const publisher = { publish: vi.fn().mockResolvedValue({}) };
    const worker = new PaymentOutboxWorker(store, publisher, { info: vi.fn(), warn: vi.fn(), error: vi.fn() });

    await worker.runOnce();

    expect(publisher.publish).toHaveBeenCalledWith("payment.succeeded", {
      paymentIntentId: "pi_test_123",
      orderId: "order-1",
      amount: 4250,
    }, "transaction-1");
    expect(store.markPublished).toHaveBeenCalledWith("event-1");
  });

  it("leaves a failed publication pending and logs the retry", async () => {
    const store = {
      claimBatch: vi.fn().mockResolvedValue([{
        eventId: "event-2",
        aggregateId: "transaction-2",
        type: "PaymentFailed",
        occurredAt: "2026-09-27T00:00:00.000Z",
        data: { stripePaymentIntentId: "pi_test_failed", orderId: "order-2", amount: 1000, reason: "declined" },
      }]),
      markPublished: vi.fn().mockResolvedValue(undefined),
    };
    const warn = vi.fn();
    const worker = new PaymentOutboxWorker(store, { publish: vi.fn().mockRejectedValue(new Error("SNS unavailable")) }, { info: vi.fn(), warn, error: vi.fn() });

    await expect(worker.runOnce()).resolves.toBeUndefined();
    expect(store.markPublished).not.toHaveBeenCalled();
    expect(warn).toHaveBeenCalledOnce();
  });
});
