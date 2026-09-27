import type Stripe from "stripe";
import type { CreatePaymentInput, PaymentStatus, PaymentTransaction } from "./domain/payment.js";

export interface StripeIntentResult {
  id: string;
  clientSecret: string | null;
  status: string;
}

export interface StripeGateway {
  createIntent(input: { orderId: string; amount: number; currency: string }, idempotencyKey: string): Promise<StripeIntentResult>;
  retrieveIntent(intentId: string): Promise<StripeIntentResult>;
  constructWebhookEvent(rawBody: Buffer, signature: string): Stripe.Event;
}

export interface PaymentRepository {
  findByIdempotencyKey(key: string): Promise<PaymentTransaction | null>;
  savePending(input: CreatePaymentInput, stripeIntentId: string): Promise<PaymentTransaction>;
  applyWebhook(input: {
    eventId: string;
    eventType: string;
    eventCreated: number;
    stripeIntentId: string;
    status: PaymentStatus;
    errorMessage: string | null;
  }): Promise<{ duplicate: boolean; transaction: PaymentTransaction }>;
}

export interface AppLogger {
  info(context: object, message: string): void;
  warn(context: object, message: string): void;
  error(context: object, message: string): void;
}
