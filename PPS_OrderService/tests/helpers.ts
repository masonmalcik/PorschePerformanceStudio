import type { Order, OrderStatus } from "../src/domain/order.js";

export function orderFixture(status: OrderStatus = "PENDING"): Order {
  return {
    id: "11111111-1111-4111-8111-111111111111",
    userId: "user-1",
    status,
    totalAmount: "20.00",
    trackingId: null,
    createdAt: "2026-01-01T00:00:00.000Z",
    updatedAt: "2026-01-01T00:00:00.000Z",
    items: [{ id: "item-1", productId: "product-1", quantity: 2, pricePerUnit: "10.00" }],
    history: [],
  };
}
