import type Stripe from "stripe";
import { describe, expect, it, vi } from "vitest";
import { OfficialStripeGateway } from "../../src/infrastructure/stripe-gateway.js";

describe("OfficialStripeGateway", () => {
  it("maps PaymentIntent fields and order metadata into the Stripe SDK", async () => {
    const create = vi.fn().mockResolvedValue({ id: "pi_123", client_secret: "secret", status: "requires_payment_method" });
    const stripeClient = { paymentIntents: { create }, webhooks: { constructEvent: vi.fn() } } as unknown as Stripe;
    const gateway = new OfficialStripeGateway("unused", "whsec_test", stripeClient);
    await gateway.createIntent({ orderId: "order-123", amount: 14999, currency: "usd" }, "checkout-123");
    expect(create).toHaveBeenCalledWith({
      amount: 14999,
      currency: "usd",
      metadata: { orderId: "order-123" },
      automatic_payment_methods: { enabled: true },
    }, { idempotencyKey: "checkout-123" });
  });
});
