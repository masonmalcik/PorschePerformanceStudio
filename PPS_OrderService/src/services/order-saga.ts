import type { DomainEvent, Order } from "../domain/order.js";
import type { OrderRepository } from "../ports.js";

export class OrderSaga {
  public constructor(private readonly repository: OrderRepository) {}

  public async inventoryReserved(event: DomainEvent): Promise<Order> {
    const orderId = this.orderId(event);
    return this.repository.transition({
      orderId,
      allowedFrom: ["PENDING"],
      to: "STOCK_RESERVED",
      reason: "Inventory reserved",
      eventId: event.eventId,
      events: [{ type: "PaymentRequested", data: { orderId } }],
    });
  }

  public async inventoryUnavailable(event: DomainEvent): Promise<Order> {
    const orderId = this.orderId(event);
    return this.repository.transition({
      orderId,
      allowedFrom: ["PENDING"],
      to: "FAILED",
      reason: String(event.data.reason ?? "Inventory unavailable"),
      eventId: event.eventId,
      events: [{ type: "OrderFailed", data: { orderId, reason: "INVENTORY_UNAVAILABLE" } }],
    });
  }

  public async paymentProcessed(event: DomainEvent): Promise<Order> {
    const orderId = this.orderId(event);
    await this.repository.transition({
      orderId,
      allowedFrom: ["STOCK_RESERVED"],
      to: "PAID",
      reason: "Payment processed",
      eventId: event.eventId,
      events: [{ type: "OrderPaid", data: { orderId, paymentId: event.data.paymentId } }],
    });
    return this.repository.transition({
      orderId,
      allowedFrom: ["PAID"],
      to: "CONFIRMED",
      reason: "Checkout saga completed",
      events: [{ type: "OrderConfirmed", data: { orderId } }],
    });
  }

  public async paymentFailed(event: DomainEvent): Promise<Order> {
    const orderId = this.orderId(event);
    return this.repository.transition({
      orderId,
      allowedFrom: ["STOCK_RESERVED"],
      to: "FAILED",
      reason: String(event.data.reason ?? "Payment failed"),
      eventId: event.eventId,
      events: [
        { type: "InventoryReleaseRequested", data: { orderId, reason: "PAYMENT_FAILED" } },
        { type: "OrderFailed", data: { orderId, reason: "PAYMENT_FAILED" } },
      ],
    });
  }

  public async inventoryReleased(event: DomainEvent): Promise<Order> {
    const orderId = this.orderId(event);
    return this.repository.transition({
      orderId,
      allowedFrom: ["FAILED", "CANCELLED"],
      to: "COMPENSATED",
      reason: "Reserved inventory released",
      eventId: event.eventId,
      events: [{ type: "OrderCompensated", data: { orderId } }],
    });
  }

  private orderId(event: DomainEvent): string {
    const value = event.data.orderId;
    if (typeof value !== "string" || value.length === 0) throw new Error(`${event.type} is missing orderId`);
    return value;
  }
}
