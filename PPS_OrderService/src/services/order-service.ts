import type { CreateOrderInput, Order } from "../domain/order.js";
import { InvalidOrderTransitionError, OrderNotFoundError } from "../domain/order.js";
import type { OrderRepository } from "../ports.js";

export class OrderService {
  public constructor(private readonly repository: OrderRepository) {}

  public async create(input: CreateOrderInput): Promise<{ order: Order; replayed: boolean }> {
    const result = await this.repository.create(input);
    if (result.order.userId !== input.userId) throw new Error("Idempotency key belongs to another principal");
    return result;
  }

  public async get(id: string, userId: string): Promise<Order> {
    const order = await this.repository.findById(id);
    if (!order || order.userId !== userId) throw new OrderNotFoundError(id);
    return order;
  }

  public list(userId: string): Promise<Order[]> {
    return this.repository.findByUserId(userId);
  }

  public async cancel(id: string, userId: string): Promise<Order> {
    const order = await this.get(id, userId);
    if (order.status === "PENDING") {
      return this.repository.transition({
        orderId: id, allowedFrom: ["PENDING"], to: "CANCELLED", reason: "Cancelled by user",
        events: [{ type: "OrderCancelled", data: { orderId: id } }],
      });
    }
    if (order.status === "STOCK_RESERVED") {
      return this.repository.transition({
        orderId: id, allowedFrom: ["STOCK_RESERVED"], to: "CANCELLED", reason: "Cancelled by user",
        events: [
          { type: "InventoryReleaseRequested", data: { orderId: id, reason: "USER_CANCELLED" } },
          { type: "OrderCancelled", data: { orderId: id } },
        ],
      });
    }
    throw new InvalidOrderTransitionError(order.status, "CANCELLED");
  }
}
