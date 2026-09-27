import pino from "pino";
import request from "supertest";
import { describe, expect, it, vi } from "vitest";
import { createApp } from "../../src/app.js";
import type { PaymentRepository, StripeGateway } from "../../src/ports.js";
import { PaymentService } from "../../src/services/payment-service.js";
import { transactionFixture } from "../helpers.js";

const log = pino({ level: "silent" });
function setup() {
  const repository: PaymentRepository = {
    findByIdempotencyKey: vi.fn().mockResolvedValue(null),
    savePending: vi.fn().mockResolvedValue(transactionFixture()),
    applyWebhook: vi.fn(),
  };
  const stripe: StripeGateway = {
    createIntent: vi.fn().mockResolvedValue({ id: "pi_test_123", clientSecret: "secret", status: "requires_payment_method" }),
    retrieveIntent: vi.fn().mockResolvedValue({ id: "pi_test_123", clientSecret: "secret", status: "requires_payment_method" }),
    constructWebhookEvent: vi.fn(),
  };
  const service = new PaymentService(repository, stripe, log);
  return { repository, stripe, app: createApp({ service, stripe, logger: log }) };
}

describe("POST /payments/create-intent", () => {
  it("maps minor-unit amounts, currency, order metadata, and Stripe idempotency", async () => {
    const { app, repository, stripe } = setup();
    const response = await request(app).post("/payments/create-intent")
      .set("x-authenticated-user-id", "user-123")
      .set("idempotency-key", "checkout-123")
      .send({ orderId: transactionFixture().orderId, amount: 14999, currency: "USD" });
    expect(response.status).toBe(201);
    expect(response.body.clientSecret).toBe("secret");
    expect(stripe.createIntent).toHaveBeenCalledWith(
      expect.objectContaining({ orderId: transactionFixture().orderId, amount: 14999, currency: "usd" }),
      "checkout-123",
    );
    expect(repository.savePending).toHaveBeenCalledWith(expect.objectContaining({ userId: "user-123" }), "pi_test_123");
  });

  it("returns 502 and does not write a ledger row when Stripe fails", async () => {
    const { app, repository, stripe } = setup();
    vi.mocked(stripe.createIntent).mockRejectedValue(new Error("network timeout"));
    const response = await request(app).post("/payments/create-intent")
      .set("x-authenticated-user-id", "user-123")
      .set("idempotency-key", "checkout-123")
      .send({ orderId: transactionFixture().orderId, amount: 14999, currency: "usd" });
    expect(response.status).toBe(502);
    expect(repository.savePending).not.toHaveBeenCalled();
  });

  it("rejects missing authentication and fractional minor-unit amounts", async () => {
    const { app } = setup();
    expect((await request(app).post("/payments/create-intent").send({})).status).toBe(401);
    const invalid = await request(app).post("/payments/create-intent")
      .set("x-authenticated-user-id", "user-123").set("idempotency-key", "checkout-123")
      .send({ orderId: transactionFixture().orderId, amount: 10.5, currency: "usd" });
    expect(invalid.status).toBe(400);
  });
});
