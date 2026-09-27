import { describe, expect, it, vi } from "vitest";
import type { OrderRepository } from "../../src/ports.js";
import { OrderService } from "../../src/services/order-service.js";
import { orderFixture } from "../helpers.js";

function repository(): OrderRepository {
  return { create: vi.fn(), findById: vi.fn(), findByUserId: vi.fn(), transition: vi.fn() };
}

describe("OrderService", () => {
  it("preserves repository idempotency replay information", async () => {
    const repo = repository();
    vi.mocked(repo.create).mockResolvedValue({ order: orderFixture(), replayed: true });
    const result = await new OrderService(repo).create({
      userId: "user-1", idempotencyKey: "checkout-1",
      items: [{ productId: "product-1", quantity: 2, pricePerUnit: "10.00" }],
    });
    expect(result.replayed).toBe(true);
  });

  it("cancels a stock-reserved order and requests inventory release", async () => {
    const repo = repository();
    vi.mocked(repo.findById).mockResolvedValue(orderFixture("STOCK_RESERVED"));
    vi.mocked(repo.transition).mockResolvedValue(orderFixture("CANCELLED"));
    await new OrderService(repo).cancel(orderFixture().id, "user-1");
    expect(repo.transition).toHaveBeenCalledWith(expect.objectContaining({
      to: "CANCELLED",
      events: expect.arrayContaining([expect.objectContaining({ type: "InventoryReleaseRequested" })]),
    }));
  });
});
