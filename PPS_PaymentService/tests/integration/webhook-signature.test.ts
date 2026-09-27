import pino from "pino";
import request from "supertest";
import Stripe from "stripe";
import { describe, expect, it } from "vitest";
import { createApp } from "../../src/app.js";
import type { PaymentStatus, PaymentTransaction } from "../../src/domain/payment.js";
import { OfficialStripeGateway } from "../../src/infrastructure/stripe-gateway.js";
import type { PaymentRepository } from "../../src/ports.js";
import { PaymentService } from "../../src/services/payment-service.js";
import { transactionFixture } from "../helpers.js";

const webhookSecret = "whsec_integration_test_secret";
const log = pino({ level: "silent" });

class LedgerHarness implements PaymentRepository {
  public transaction: PaymentTransaction = transactionFixture();
  public readonly processed = new Set<string>();
  public externalEvents = 0;
  public findByIdempotencyKey(): Promise<PaymentTransaction | null> { return Promise.resolve(null); }
  public savePending(): Promise<PaymentTransaction> { return Promise.resolve(this.transaction); }
  public applyWebhook(input: {
    eventId: string; eventType: string; eventCreated: number; stripeIntentId: string;
    status: PaymentStatus; errorMessage: string | null;
  }): Promise<{ duplicate: boolean; transaction: PaymentTransaction }> {
    if (this.processed.has(input.eventId)) return Promise.resolve({ duplicate: true, transaction: this.transaction });
    this.processed.add(input.eventId);
    this.transaction = { ...this.transaction, status: input.status, errorMessage: input.errorMessage };
    this.externalEvents++;
    return Promise.resolve({ duplicate: false, transaction: this.transaction });
  }
}

function signedEvent(type: "payment_intent.succeeded" | "payment_intent.payment_failed", eventId: string): { payload: string; signature: string } {
  const payload = JSON.stringify({
    id: eventId,
    object: "event",
    api_version: "2025-12-15.clover",
    created: 1_800_000_000,
    type,
    data: { object: {
      id: "pi_test_123",
      object: "payment_intent",
      metadata: { orderId: transactionFixture().orderId },
      last_payment_error: type === "payment_intent.payment_failed" ? { message: "Card declined" } : null,
    } },
  });
  return { payload, signature: Stripe.webhooks.generateTestHeaderString({ payload, secret: webhookSecret }) };
}

function setup() {
  const repository = new LedgerHarness();
  const stripe = new OfficialStripeGateway("sk_test_placeholder", webhookSecret);
  const service = new PaymentService(repository, stripe, log);
  return { repository, app: createApp({ service, stripe, logger: log }) };
}

describe("Stripe webhook signature and ledger integration", () => {
  it.each([
    ["payment_intent.succeeded", "SUCCEEDED", null],
    ["payment_intent.payment_failed", "FAILED", "Card declined"],
  ] as const)("accepts a genuine %s event", async (type, status, errorMessage) => {
    const { app, repository } = setup();
    const event = signedEvent(type, `evt_${status.toLowerCase()}`);
    const response = await request(app).post("/payments/webhook")
      .set("Content-Type", "application/json").set("stripe-signature", event.signature).send(event.payload);
    expect(response.status).toBe(200);
    expect(repository.transaction.status).toBe(status);
    expect(repository.transaction.errorMessage).toBe(errorMessage);
  });

  it("accepts a duplicate delivery without repeating its external event", async () => {
    const { app, repository } = setup();
    const event = signedEvent("payment_intent.succeeded", "evt_duplicate");
    const first = await request(app).post("/payments/webhook").set("Content-Type", "application/json").set("stripe-signature", event.signature).send(event.payload);
    const second = await request(app).post("/payments/webhook").set("Content-Type", "application/json").set("stripe-signature", event.signature).send(event.payload);
    expect(first.status).toBe(200);
    expect(second.status).toBe(200);
    expect(second.body.duplicate).toBe(true);
    expect(repository.externalEvents).toBe(1);
  });

  it("rejects missing and invalid signatures", async () => {
    const { app } = setup();
    const event = signedEvent("payment_intent.succeeded", "evt_invalid");
    expect((await request(app).post("/payments/webhook").set("Content-Type", "application/json").send(event.payload)).status).toBe(400);
    expect((await request(app).post("/payments/webhook").set("Content-Type", "application/json").set("stripe-signature", "bad").send(event.payload)).status).toBe(400);
  });
});
