import type { PaymentTransaction } from "../src/domain/payment.js";
import type { AppLogger } from "../src/ports.js";

export const logger: AppLogger = { info: () => undefined, warn: () => undefined, error: () => undefined };
export const transactionFixture = (status: PaymentTransaction["status"] = "PENDING"): PaymentTransaction => ({
  id: "22222222-2222-4222-8222-222222222222",
  orderId: "11111111-1111-4111-8111-111111111111",
  userId: "user-123",
  stripePaymentIntentId: "pi_test_123",
  amount: 14999,
  currency: "usd",
  status,
  errorMessage: null,
  createdAt: "2026-01-01T00:00:00.000Z",
  updatedAt: "2026-01-01T00:00:00.000Z",
});
