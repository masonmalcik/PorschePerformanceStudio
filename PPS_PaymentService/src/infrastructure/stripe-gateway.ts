import Stripe from "stripe";
import type { StripeGateway, StripeIntentResult } from "../ports.js";

export class OfficialStripeGateway implements StripeGateway {
  private readonly stripe: Stripe;

  public constructor(secretKey: string, private readonly webhookSecret: string, stripeClient?: Stripe) {
    this.stripe = stripeClient ?? new Stripe(secretKey, { maxNetworkRetries: 2, timeout: 10_000 });
  }

  public async retrieveIntent(intentId: string): Promise<StripeIntentResult> {
    const intent = await this.stripe.paymentIntents.retrieve(intentId);
    return { id: intent.id, clientSecret: intent.client_secret, status: intent.status };
  }

  public async createIntent(
    input: { orderId: string; amount: number; currency: string },
    idempotencyKey: string,
  ): Promise<StripeIntentResult> {
    const intent = await this.stripe.paymentIntents.create({
      amount: input.amount,
      currency: input.currency,
      metadata: { orderId: input.orderId },
      automatic_payment_methods: { enabled: true },
    }, { idempotencyKey });
    return { id: intent.id, clientSecret: intent.client_secret, status: intent.status };
  }

  public constructWebhookEvent(rawBody: Buffer, signature: string): Stripe.Event {
    return this.stripe.webhooks.constructEvent(rawBody, signature, this.webhookSecret);
  }
}
