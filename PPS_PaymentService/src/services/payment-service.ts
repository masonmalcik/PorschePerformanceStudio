import type Stripe from "stripe";
import { IdempotencyConflictError, PaymentProviderError, type CreatePaymentInput, type PaymentTransaction } from "../domain/payment.js";
import type { AppLogger, PaymentRepository, StripeGateway } from "../ports.js";

export class PaymentService {
  public constructor(
    private readonly repository: PaymentRepository,
    private readonly stripe: StripeGateway,
    private readonly logger: AppLogger,
  ) {}

  public async createIntent(input: CreatePaymentInput): Promise<{
    transaction: PaymentTransaction;
    clientSecret: string | null;
    replayed: boolean;
  }> {
    const existing = await this.repository.findByIdempotencyKey(input.idempotencyKey);
    if (existing) {
      if (existing.orderId !== input.orderId || existing.userId !== input.userId || existing.amount !== input.amount || existing.currency !== input.currency) {
        throw new IdempotencyConflictError();
      }
      const intent = await this.stripe.retrieveIntent(existing.stripePaymentIntentId);
      return { transaction: existing, clientSecret: intent.clientSecret, replayed: true };
    }
    try {
      const intent = await this.stripe.createIntent(input, input.idempotencyKey);
      const transaction = await this.repository.savePending(input, intent.id);
      return { transaction, clientSecret: intent.clientSecret, replayed: false };
    } catch (error) {
      this.logger.error({ error, orderId: input.orderId }, "Stripe PaymentIntent creation failed");
      if (error instanceof IdempotencyConflictError) throw error;
      throw new PaymentProviderError();
    }
  }

  public async processWebhook(event: Stripe.Event): Promise<{ duplicate: boolean }> {
    if (event.livemode) {
      this.logger.warn({ eventId: event.id }, "Rejected live-mode Stripe event in the PPS demo");
      throw new PaymentProviderError();
    }
    if (event.type !== "payment_intent.succeeded" && event.type !== "payment_intent.payment_failed") {
      return { duplicate: false };
    }
    const intent = event.data.object as Stripe.PaymentIntent;
    const succeeded = event.type === "payment_intent.succeeded";
    const result = await this.repository.applyWebhook({
      eventId: event.id,
      eventType: event.type,
      eventCreated: event.created,
      stripeIntentId: intent.id,
      status: succeeded ? "SUCCEEDED" : "FAILED",
      errorMessage: succeeded ? null : (intent.last_payment_error?.message ?? "Payment failed"),
    });
    return { duplicate: result.duplicate };
  }
}
