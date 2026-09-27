import type { EventEnvelope, EventPayloads, RoutingKey } from "@pps/messaging";
import type { DomainEvent } from "../domain/order.js";
import type { MessagePublisher } from "../ports.js";

export interface TypedEventPublisher {
  publish<K extends RoutingKey>(routingKey: K, payload: EventPayloads[K], correlationId?: string): Promise<EventEnvelope<K>>;
}

export class CheckoutPublisher implements MessagePublisher {
  public constructor(private readonly publisher: TypedEventPublisher) {}
  public async publish(event: DomainEvent): Promise<void> {
    const data = event.data;
    switch (event.type) {
      case "OrderCreated": await this.publisher.publish("order.created", { orderId: required(data.orderId), userId: required(data.userId), totalAmount: required(data.totalAmount), items: list(data.items) as EventPayloads["order.created"]["items"] }, event.aggregateId); return;
      case "PaymentRequested": await this.publisher.publish("payment.requested", { orderId: required(data.orderId), userId: required(data.userId), totalAmount: required(data.totalAmount), currency: "usd" }, event.aggregateId); return;
      case "InventoryReleaseRequested": await this.publisher.publish("inventory.release-requested", { orderId: required(data.orderId), reason: required(data.reason) }, event.aggregateId); return;
      case "OrderConfirmed": await this.publisher.publish("order.confirmed", { orderId: required(data.orderId) }, event.aggregateId); return;
      case "OrderFailed": await this.publisher.publish("order.failed", { orderId: required(data.orderId), reason: required(data.reason) }, event.aggregateId); return;
      case "OrderCancelled": await this.publisher.publish("order.cancelled", { orderId: required(data.orderId) }, event.aggregateId); return;
      case "OrderCompensated": await this.publisher.publish("order.compensated", { orderId: required(data.orderId) }, event.aggregateId); return;
      case "OrderPaid": return;
      default: throw new Error(`Unsupported Order outbox event: ${event.type}`);
    }
  }
}
function required(value: unknown): string { if (typeof value !== "string" || !value) throw new Error("Event field must be a non-empty string"); return value; }
function list(value: unknown): unknown[] { if (!Array.isArray(value)) throw new Error("Event field must be an array"); return value; }
