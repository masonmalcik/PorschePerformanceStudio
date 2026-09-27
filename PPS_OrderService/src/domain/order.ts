export const orderStatuses = ["PENDING", "STOCK_RESERVED", "PAID", "CONFIRMED", "CANCELLED", "FAILED", "COMPENSATED"] as const;
export type OrderStatus = (typeof orderStatuses)[number];

export interface OrderItem {
  id: string;
  productId: string;
  quantity: number;
  pricePerUnit: string;
}

export interface OrderHistoryEntry {
  id: string;
  fromStatus: OrderStatus | null;
  toStatus: OrderStatus;
  reason: string | null;
  eventId: string | null;
  createdAt: string;
}

export interface Order {
  id: string;
  userId: string;
  status: OrderStatus;
  totalAmount: string;
  trackingId: string | null;
  createdAt: string;
  updatedAt: string;
  items: OrderItem[];
  history: OrderHistoryEntry[];
}

export interface CreateOrderInput {
  userId: string;
  idempotencyKey: string;
  items: Array<{ productId: string; quantity: number; pricePerUnit: string }>;
}

export interface DomainEvent<T = Record<string, unknown>> {
  eventId: string;
  type: string;
  aggregateId: string;
  occurredAt: string;
  data: T;
}

export class InvalidOrderTransitionError extends Error {
  public constructor(public readonly status: OrderStatus, target: OrderStatus) {
    super(`Cannot transition order from ${status} to ${target}`);
    this.name = "InvalidOrderTransitionError";
  }
}

export class OrderNotFoundError extends Error {
  public constructor(id: string) {
    super(`Order ${id} was not found`);
    this.name = "OrderNotFoundError";
  }
}
