import { describe, expect, it, vi } from "vitest";
import type { DomainEvent, OrderStatus } from "../../src/domain/order.js";
import type { OrderRepository } from "../../src/ports.js";
import { OrderSaga } from "../../src/services/order-saga.js";
import { orderFixture } from "../helpers.js";

function event(type: string, data: Record<string, unknown> = {}): DomainEvent {
  return { eventId: `${type}-1`, type, aggregateId: "order-1", occurredAt: new Date().toISOString(), data: { orderId: "order-1", ...data } };
}

function statefulRepository(initial: OrderStatus): OrderRepository & { transition: ReturnType<typeof vi.fn> } {
  let status = initial;
  return {
    create: vi.fn(), findById: vi.fn(), findByUserId: vi.fn(),
    transition: vi.fn(async (input: { allowedFrom: OrderStatus[]; to: OrderStatus }) => {
      expect(input.allowedFrom).toContain(status);
      status = input.to;
      return orderFixture(status);
    }),
  };
}

describe("OrderSaga", () => {
  it("executes STOCK_RESERVED -> PAID -> CONFIRMED on the happy path", async () => {
    const repository = statefulRepository("PENDING");
    const saga = new OrderSaga(repository);
    await saga.inventoryReserved(event("InventoryReserved"));
    const result = await saga.paymentProcessed(event("PaymentProcessed", { paymentId: "payment-1" }));
    expect(result.status).toBe("CONFIRMED");
    expect(repository.transition).toHaveBeenNthCalledWith(1, expect.objectContaining({ to: "STOCK_RESERVED", events: [expect.objectContaining({ type: "PaymentRequested" })] }));
    expect(repository.transition).toHaveBeenNthCalledWith(2, expect.objectContaining({ to: "PAID" }));
    expect(repository.transition).toHaveBeenNthCalledWith(3, expect.objectContaining({ to: "CONFIRMED", events: [expect.objectContaining({ type: "OrderConfirmed" })] }));
  });

  it("fails payment and requests inventory compensation", async () => {
    const repository = statefulRepository("STOCK_RESERVED");
    const result = await new OrderSaga(repository).paymentFailed(event("PaymentFailed", { reason: "declined" }));
    expect(result.status).toBe("FAILED");
    expect(repository.transition).toHaveBeenCalledWith(expect.objectContaining({
      to: "FAILED",
      events: expect.arrayContaining([expect.objectContaining({ type: "InventoryReleaseRequested" })]),
    }));
  });

  it("compensates after inventory release", async () => {
    const repository = statefulRepository("FAILED");
    await expect(new OrderSaga(repository).inventoryReleased(event("InventoryReleased")))
      .resolves.toEqual(expect.objectContaining({ status: "COMPENSATED" }));
  });

  it("fails without payment when inventory is unavailable", async () => {
    const repository = statefulRepository("PENDING");
    await expect(new OrderSaga(repository).inventoryUnavailable(event("InventoryUnavailable")))
      .resolves.toEqual(expect.objectContaining({ status: "FAILED" }));
  });
});
