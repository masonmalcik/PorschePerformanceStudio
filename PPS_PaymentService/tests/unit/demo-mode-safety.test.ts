import type Stripe from "stripe";
import { describe, expect, it, vi } from "vitest";
import { loadConfig } from "../../src/config.js";
import { PaymentProviderError } from "../../src/domain/payment.js";
import type { PaymentRepository, StripeGateway } from "../../src/ports.js";
import { PaymentService } from "../../src/services/payment-service.js";
import { logger } from "../helpers.js";

describe("Stripe demo-mode safety", () => {
  it("rejects live Stripe API keys during configuration", () => {
    expect(() => loadConfig({
      DATABASE_URL: "postgresql://localhost/test",
      STRIPE_SECRET_KEY: "sk_live_forbidden",
      STRIPE_WEBHOOK_SECRET: "whsec_test",
    })).toThrow(/test-mode secret key/);
  });

  it("rejects live-mode webhook events before touching the ledger", async () => {
    const repository = { applyWebhook: vi.fn() } as unknown as PaymentRepository;
    const stripe = {} as StripeGateway;
    const service = new PaymentService(repository, stripe, logger);
    const event = { id: "evt_live", livemode: true, type: "payment_intent.succeeded" } as Stripe.Event;

    await expect(service.processWebhook(event)).rejects.toBeInstanceOf(PaymentProviderError);
    expect(repository.applyWebhook).not.toHaveBeenCalled();
  });
});
