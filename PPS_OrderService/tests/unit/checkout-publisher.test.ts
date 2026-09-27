import { describe, expect, it, vi } from "vitest";
import { CheckoutPublisher } from "../../src/messaging/checkout-publisher.js";

describe("CheckoutPublisher", () => {
  it("maps a payment request outbox event to the shared contract", async () => {
    const publish = vi.fn().mockResolvedValue({});
    const publisher = new CheckoutPublisher({ publish });

    await publisher.publish({
      eventId: "event-1",
      type: "PaymentRequested",
      aggregateId: "order-1",
      occurredAt: "2026-09-27T00:00:00.000Z",
      data: { orderId: "order-1", userId: "user-1", totalAmount: "42.50" },
    });

    expect(publish).toHaveBeenCalledWith("payment.requested", {
      orderId: "order-1",
      userId: "user-1",
      totalAmount: "42.50",
      currency: "usd",
    }, "order-1");
  });

  it("rejects malformed durable events instead of publishing invalid messages", async () => {
    const publisher = new CheckoutPublisher({ publish: vi.fn() });
    await expect(publisher.publish({
      eventId: "event-2",
      type: "OrderFailed",
      aggregateId: "order-1",
      occurredAt: "2026-09-27T00:00:00.000Z",
      data: { orderId: "order-1" },
    })).rejects.toThrow("Event field must be a non-empty string");
  });
});
