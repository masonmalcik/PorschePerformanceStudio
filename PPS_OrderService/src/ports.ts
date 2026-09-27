import type { CreateOrderInput, DomainEvent, Order, OrderStatus } from "./domain/order.js";

export interface OutgoingEvent {
  type: string;
  data: Record<string, unknown>;
}

export interface OrderRepository {
  create(input: CreateOrderInput): Promise<{ order: Order; replayed: boolean }>;
  findById(id: string): Promise<Order | null>;
  findByUserId(userId: string): Promise<Order[]>;
  transition(input: {
    orderId: string;
    allowedFrom: OrderStatus[];
    to: OrderStatus;
    reason?: string;
    eventId?: string;
    events: OutgoingEvent[];
  }): Promise<Order>;
}

export interface MessagePublisher {
  publish(event: DomainEvent): Promise<void>;
}

export type MessageHandler = (event: DomainEvent) => Promise<void>;

export interface MessageConsumer {
  subscribe(eventType: string, handler: MessageHandler): Promise<() => Promise<void>>;
}

export interface OutboxStore {
  claimBatch(limit: number): Promise<DomainEvent[]>;
  markPublished(eventId: string): Promise<void>;
  markFailed(eventId: string, retryAt: Date): Promise<void>;
}

export interface AppLogger {
  info(context: object, message: string): void;
  warn(context: object, message: string): void;
  error(context: object, message: string): void;
}
