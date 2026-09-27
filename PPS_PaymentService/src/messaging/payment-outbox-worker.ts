import type { EventEnvelope, EventPayloads, RoutingKey } from "@pps/messaging";
import type { AppLogger, PaymentOutboxEvent, PaymentOutboxStore } from "../ports.js";
interface Publisher { publish<K extends RoutingKey>(key: K, payload: EventPayloads[K], correlationId?: string): Promise<EventEnvelope<K>> }
export class PaymentOutboxWorker {
  private timer: NodeJS.Timeout | undefined;
  private running = false;
  public constructor(private readonly store: PaymentOutboxStore, private readonly publisher: Publisher, private readonly logger: AppLogger) {}
  public start(intervalMs = 1000): void { this.timer ??= setInterval(() => void this.runOnce(), intervalMs); this.timer.unref(); }
  public stop(): void { if (this.timer) clearInterval(this.timer); this.timer = undefined; }
  public async runOnce(): Promise<void> {
    if (this.running) return;
    this.running = true;
    try {
      for (const event of await this.store.claimBatch(50)) {
        try {
          await this.publish(event);
          await this.store.markPublished(event.eventId);
        } catch (error) {
          this.logger.warn({ error, eventId: event.eventId }, "Payment outbox publish failed; it will be retried");
        }
      }
    } finally {
      this.running = false;
    }
  }
  private async publish(event: PaymentOutboxEvent): Promise<void> { const success = event.type === "PaymentProcessed"; if (!success && event.type !== "PaymentFailed") throw new Error(`Unsupported Payment outbox event: ${event.type}`); const base = { paymentIntentId: required(event.data.stripePaymentIntentId), orderId: required(event.data.orderId), amount: integer(event.data.amount) }; if (success) await this.publisher.publish("payment.succeeded", base, event.aggregateId); else await this.publisher.publish("payment.failed", { ...base, ...(typeof event.data.reason === "string" ? { reason: event.data.reason } : {}) }, event.aggregateId); }
}
function required(value: unknown): string { if (typeof value !== "string" || !value) throw new Error("Payment event field is required"); return value; }
function integer(value: unknown): number { if (typeof value !== "number" || !Number.isInteger(value) || value <= 0) throw new Error("Payment amount must be a positive integer"); return value; }
