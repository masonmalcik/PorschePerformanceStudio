CREATE TYPE "PaymentStatus" AS ENUM ('PENDING', 'SUCCEEDED', 'FAILED');

CREATE TABLE "payment_transactions" (
  "id" UUID NOT NULL DEFAULT gen_random_uuid(),
  "order_id" UUID NOT NULL,
  "user_id" VARCHAR(128) NOT NULL,
  "stripe_payment_intent_id" VARCHAR(255) NOT NULL,
  "idempotency_key" VARCHAR(128) NOT NULL,
  "amount" INTEGER NOT NULL,
  "currency" CHAR(3) NOT NULL,
  "status" "PaymentStatus" NOT NULL DEFAULT 'PENDING',
  "error_message" VARCHAR(1000),
  "last_event_created" INTEGER NOT NULL DEFAULT 0,
  "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  "updated_at" TIMESTAMP(3) NOT NULL,
  CONSTRAINT "payment_transactions_pkey" PRIMARY KEY ("id"),
  CONSTRAINT "payment_transactions_amount_check" CHECK ("amount" > 0),
  CONSTRAINT "payment_transactions_currency_check" CHECK ("currency" ~ '^[a-z]{3}$')
);

CREATE TABLE "stripe_webhook_events" (
  "event_id" VARCHAR(255) NOT NULL,
  "event_type" VARCHAR(100) NOT NULL,
  "processed_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT "stripe_webhook_events_pkey" PRIMARY KEY ("event_id")
);

CREATE TABLE "payment_outbox_events" (
  "id" UUID NOT NULL DEFAULT gen_random_uuid(),
  "aggregate_id" UUID NOT NULL,
  "type" VARCHAR(100) NOT NULL,
  "payload" JSONB NOT NULL,
  "published_at" TIMESTAMP(3),
  "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT "payment_outbox_events_pkey" PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "payment_transactions_stripe_payment_intent_id_key" ON "payment_transactions"("stripe_payment_intent_id");
CREATE UNIQUE INDEX "payment_transactions_idempotency_key_key" ON "payment_transactions"("idempotency_key");
CREATE INDEX "payment_transactions_order_id_idx" ON "payment_transactions"("order_id");
CREATE INDEX "payment_transactions_user_id_created_at_idx" ON "payment_transactions"("user_id", "created_at" DESC);
CREATE INDEX "payment_outbox_events_published_at_created_at_idx" ON "payment_outbox_events"("published_at", "created_at");
