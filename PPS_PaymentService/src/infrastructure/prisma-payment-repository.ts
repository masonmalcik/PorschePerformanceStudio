import { PaymentStatus as PrismaStatus, Prisma, PrismaClient } from "@prisma/client";
import type { CreatePaymentInput, PaymentStatus, PaymentTransaction } from "../domain/payment.js";
import { UnknownPaymentIntentError } from "../domain/payment.js";
import type { PaymentOutboxStore, PaymentRepository } from "../ports.js";

type PaymentRecord = Prisma.PaymentTransactionGetPayload<Record<string, never>>;

export class PrismaPaymentRepository implements PaymentRepository, PaymentOutboxStore {
  public constructor(private readonly prisma: PrismaClient) {}

  public async findByIdempotencyKey(key: string): Promise<PaymentTransaction | null> {
    const record = await this.prisma.paymentTransaction.findUnique({ where: { idempotencyKey: key } });
    return record ? this.map(record) : null;
  }

  public async savePending(input: CreatePaymentInput, stripeIntentId: string): Promise<PaymentTransaction> {
    try {
      return this.map(await this.prisma.paymentTransaction.create({
        data: {
          orderId: input.orderId,
          userId: input.userId,
          stripePaymentIntentId: stripeIntentId,
          idempotencyKey: input.idempotencyKey,
          amount: input.amount,
          currency: input.currency,
        },
      }));
    } catch (error) {
      if (error instanceof Prisma.PrismaClientKnownRequestError && error.code === "P2002") {
        const existing = await this.prisma.paymentTransaction.findUnique({ where: { idempotencyKey: input.idempotencyKey } });
        if (existing) return this.map(existing);
      }
      throw error;
    }
  }

  public async applyWebhook(input: {
    eventId: string;
    eventType: string;
    eventCreated: number;
    stripeIntentId: string;
    status: PaymentStatus;
    errorMessage: string | null;
  }): Promise<{ duplicate: boolean; transaction: PaymentTransaction }> {
    try {
      const transaction = await this.prisma.$transaction(async (tx) => {
        const current = await tx.paymentTransaction.findUnique({ where: { stripePaymentIntentId: input.stripeIntentId } });
        if (!current) throw new UnknownPaymentIntentError(input.stripeIntentId);
        await tx.stripeWebhookEvent.create({ data: { eventId: input.eventId, eventType: input.eventType } });

        const staleFailure = input.status === "FAILED" && input.eventCreated < current.lastEventCreated;
        const terminalSuccess = current.status === PrismaStatus.SUCCEEDED;
        const unchanged = current.status === input.status;
        if (staleFailure || terminalSuccess || unchanged) return current;

        const updated = await tx.paymentTransaction.update({
          where: { id: current.id },
          data: {
            status: input.status as PrismaStatus,
            errorMessage: input.errorMessage,
            lastEventCreated: Math.max(current.lastEventCreated, input.eventCreated),
          },
        });
        await tx.paymentOutboxEvent.create({
          data: {
            aggregateId: current.orderId,
            type: input.status === "SUCCEEDED" ? "PaymentProcessed" : "PaymentFailed",
            payload: {
              orderId: current.orderId,
              paymentId: current.id,
              stripePaymentIntentId: current.stripePaymentIntentId,
              amount: current.amount,
              ...(input.errorMessage ? { reason: input.errorMessage } : {}),
            },
          },
        });
        return updated;
      }, { isolationLevel: Prisma.TransactionIsolationLevel.Serializable });
      return { duplicate: false, transaction: this.map(transaction) };
    } catch (error) {
      if (error instanceof Prisma.PrismaClientKnownRequestError && error.code === "P2002") {
        const existing = await this.prisma.paymentTransaction.findUnique({ where: { stripePaymentIntentId: input.stripeIntentId } });
        if (existing) return { duplicate: true, transaction: this.map(existing) };
      }
      throw error;
    }
  }

  public async claimBatch(limit: number) {
    const events = await this.prisma.paymentOutboxEvent.findMany({ where: { publishedAt: null }, orderBy: { createdAt: "asc" }, take: limit });
    return events.map((event) => ({ eventId: event.id, aggregateId: event.aggregateId, type: event.type, occurredAt: event.createdAt.toISOString(), data: event.payload as Record<string, unknown> }));
  }

  public async markPublished(eventId: string): Promise<void> {
    await this.prisma.paymentOutboxEvent.update({ where: { id: eventId }, data: { publishedAt: new Date() } });
  }

  private map(record: PaymentRecord): PaymentTransaction {
    return {
      id: record.id,
      orderId: record.orderId,
      userId: record.userId,
      stripePaymentIntentId: record.stripePaymentIntentId,
      amount: record.amount,
      currency: record.currency.trim(),
      status: record.status as PaymentStatus,
      errorMessage: record.errorMessage,
      createdAt: record.createdAt.toISOString(),
      updatedAt: record.updatedAt.toISOString(),
    };
  }
}
