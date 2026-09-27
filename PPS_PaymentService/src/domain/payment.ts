export type PaymentStatus = "PENDING" | "SUCCEEDED" | "FAILED";

export interface PaymentTransaction {
  id: string;
  orderId: string;
  userId: string;
  stripePaymentIntentId: string;
  amount: number;
  currency: string;
  status: PaymentStatus;
  errorMessage: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreatePaymentInput {
  orderId: string;
  userId: string;
  amount: number;
  currency: string;
  idempotencyKey: string;
}

export class PaymentProviderError extends Error {
  public constructor(message = "Payment provider request failed") {
    super(message);
    this.name = "PaymentProviderError";
  }
}

export class UnknownPaymentIntentError extends Error {
  public constructor(intentId: string) {
    super(`No ledger transaction exists for ${intentId}`);
    this.name = "UnknownPaymentIntentError";
  }
}

export class IdempotencyConflictError extends Error {
  public constructor() {
    super("Idempotency key was reused with a different payment request");
    this.name = "IdempotencyConflictError";
  }
}
