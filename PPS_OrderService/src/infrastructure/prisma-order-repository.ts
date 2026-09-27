import { randomUUID } from "node:crypto";
import { OrderStatus as PrismaStatus, Prisma, PrismaClient } from "@prisma/client";
import type { CreateOrderInput, DomainEvent, Order, OrderStatus } from "../domain/order.js";
import { InvalidOrderTransitionError, OrderNotFoundError } from "../domain/order.js";
import type { OrderRepository, OutboxStore } from "../ports.js";

const includeOrder = {
  items: { orderBy: { id: "asc" as const } },
  history: { orderBy: { createdAt: "asc" as const } },
};
type OrderRecord = Prisma.OrderGetPayload<{ include: typeof includeOrder }>;

export class PrismaOrderRepository implements OrderRepository, OutboxStore {
  public constructor(private readonly prisma: PrismaClient) {}

  public async create(input: CreateOrderInput): Promise<{ order: Order; replayed: boolean }> {
    const existing = await this.prisma.order.findUnique({ where: { idempotencyKey: input.idempotencyKey }, include: includeOrder });
    if (existing) return { order: this.map(existing), replayed: true };

    const total = input.items.reduce(
      (sum, item) => sum.plus(new Prisma.Decimal(item.pricePerUnit).times(item.quantity)),
      new Prisma.Decimal(0),
    );
    try {
      const record = await this.prisma.$transaction(async (tx) => {
        const order = await tx.order.create({
          data: {
            userId: input.userId,
            idempotencyKey: input.idempotencyKey,
            totalAmount: total,
            items: { create: input.items.map((item) => ({ ...item, pricePerUnit: new Prisma.Decimal(item.pricePerUnit) })) },
          },
        });
        await tx.orderHistory.create({ data: { orderId: order.id, toStatus: PrismaStatus.PENDING, reason: "Order placed" } });
        await tx.outboxEvent.create({
          data: {
            aggregateId: order.id,
            type: "OrderCreated",
            payload: { orderId: order.id, userId: order.userId, items: input.items, totalAmount: total.toFixed(2) },
          },
        });
        return tx.order.findUniqueOrThrow({ where: { id: order.id }, include: includeOrder });
      });
      return { order: this.map(record), replayed: false };
    } catch (error) {
      if (error instanceof Prisma.PrismaClientKnownRequestError && error.code === "P2002") {
        const replay = await this.prisma.order.findUnique({ where: { idempotencyKey: input.idempotencyKey }, include: includeOrder });
        if (replay) return { order: this.map(replay), replayed: true };
      }
      throw error;
    }
  }

  public async findById(id: string): Promise<Order | null> {
    const record = await this.prisma.order.findUnique({ where: { id }, include: includeOrder });
    return record ? this.map(record) : null;
  }

  public async findByUserId(userId: string): Promise<Order[]> {
    const records = await this.prisma.order.findMany({ where: { userId }, include: includeOrder, orderBy: { createdAt: "desc" } });
    return records.map((record) => this.map(record));
  }

  public async transition(input: {
    orderId: string;
    allowedFrom: OrderStatus[];
    to: OrderStatus;
    reason?: string;
    eventId?: string;
    events: Array<{ type: string; data: Record<string, unknown> }>;
  }): Promise<Order> {
    return this.prisma.$transaction(async (tx) => {
      if (input.eventId) {
        const processed = await tx.processedEvent.findUnique({ where: { eventId: input.eventId } });
        if (processed) return this.requireOrder(tx, input.orderId);
      }
      const current = await tx.order.findUnique({ where: { id: input.orderId } });
      if (!current) throw new OrderNotFoundError(input.orderId);
      const from = current.status as OrderStatus;
      if (!input.allowedFrom.includes(from)) throw new InvalidOrderTransitionError(from, input.to);
      const updated = await tx.order.updateMany({
        where: { id: current.id, version: current.version },
        data: { status: input.to as PrismaStatus, version: { increment: 1 } },
      });
      if (updated.count !== 1) throw new Error("Concurrent order update detected; event may be retried");
      await tx.orderHistory.create({
        data: {
          orderId: current.id,
          fromStatus: current.status,
          toStatus: input.to as PrismaStatus,
          reason: input.reason ?? null,
          eventId: input.eventId ?? null,
        },
      });
      if (input.events.length > 0) {
        await tx.outboxEvent.createMany({
          data: input.events.map((event) => ({
            aggregateId: current.id,
            type: event.type,
            payload: JSON.parse(JSON.stringify(event.type === "PaymentRequested" ? {
              ...event.data, orderId: current.id, userId: current.userId,
              totalAmount: current.totalAmount.toFixed(2), currency: "usd",
            } : event.data)) as Prisma.InputJsonValue,
          })),
        });
      }
      if (input.eventId) {
        await tx.processedEvent.create({ data: { eventId: input.eventId, eventType: input.to } });
      }
      return this.requireOrder(tx, current.id);
    });
  }

  public async claimBatch(limit: number): Promise<DomainEvent[]> {
    const records = await this.prisma.outboxEvent.findMany({
      where: { publishedAt: null, availableAt: { lte: new Date() } },
      orderBy: { createdAt: "asc" },
      take: limit,
    });
    return records.map((record) => ({
      eventId: record.id,
      type: record.type,
      aggregateId: record.aggregateId,
      occurredAt: record.createdAt.toISOString(),
      data: record.payload as Record<string, unknown>,
    }));
  }

  public async markPublished(eventId: string): Promise<void> {
    await this.prisma.outboxEvent.update({ where: { id: eventId }, data: { publishedAt: new Date() } });
  }

  public async markFailed(eventId: string, retryAt: Date): Promise<void> {
    await this.prisma.outboxEvent.update({ where: { id: eventId }, data: { attempts: { increment: 1 }, availableAt: retryAt } });
  }

  private requireOrder(tx: Prisma.TransactionClient, id: string): Promise<Order> {
    return tx.order.findUniqueOrThrow({ where: { id }, include: includeOrder }).then((record) => this.map(record));
  }

  private map(record: OrderRecord): Order {
    return {
      id: record.id,
      userId: record.userId,
      status: record.status as OrderStatus,
      totalAmount: record.totalAmount.toFixed(2),
      trackingId: record.trackingId,
      createdAt: record.createdAt.toISOString(),
      updatedAt: record.updatedAt.toISOString(),
      items: record.items.map((item) => ({ id: item.id, productId: item.productId, quantity: item.quantity, pricePerUnit: item.pricePerUnit.toFixed(2) })),
      history: record.history.map((entry) => ({
        id: entry.id,
        fromStatus: entry.fromStatus as OrderStatus | null,
        toStatus: entry.toStatus as OrderStatus,
        reason: entry.reason,
        eventId: entry.eventId,
        createdAt: entry.createdAt.toISOString(),
      })),
    };
  }
}
