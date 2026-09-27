CREATE TYPE "OrderStatus" AS ENUM ('PENDING', 'STOCK_RESERVED', 'PAID', 'CONFIRMED', 'CANCELLED', 'FAILED', 'COMPENSATED');

CREATE TABLE "orders" (
  "id" UUID NOT NULL DEFAULT gen_random_uuid(),
  "user_id" VARCHAR(128) NOT NULL,
  "status" "OrderStatus" NOT NULL DEFAULT 'PENDING',
  "total_amount" DECIMAL(14,2) NOT NULL,
  "tracking_id" VARCHAR(128),
  "idempotency_key" VARCHAR(128) NOT NULL,
  "version" INTEGER NOT NULL DEFAULT 0,
  "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  "updated_at" TIMESTAMP(3) NOT NULL,
  CONSTRAINT "orders_pkey" PRIMARY KEY ("id"),
  CONSTRAINT "orders_total_amount_check" CHECK ("total_amount" >= 0),
  CONSTRAINT "orders_version_check" CHECK ("version" >= 0)
);

CREATE TABLE "order_items" (
  "id" UUID NOT NULL DEFAULT gen_random_uuid(),
  "order_id" UUID NOT NULL,
  "product_id" VARCHAR(128) NOT NULL,
  "quantity" INTEGER NOT NULL,
  "price_per_unit" DECIMAL(14,2) NOT NULL,
  CONSTRAINT "order_items_pkey" PRIMARY KEY ("id"),
  CONSTRAINT "order_items_quantity_check" CHECK ("quantity" > 0),
  CONSTRAINT "order_items_price_check" CHECK ("price_per_unit" >= 0)
);

CREATE TABLE "order_history" (
  "id" UUID NOT NULL DEFAULT gen_random_uuid(),
  "order_id" UUID NOT NULL,
  "from_status" "OrderStatus",
  "to_status" "OrderStatus" NOT NULL,
  "reason" VARCHAR(500),
  "event_id" VARCHAR(128),
  "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT "order_history_pkey" PRIMARY KEY ("id")
);

CREATE TABLE "outbox_events" (
  "id" UUID NOT NULL DEFAULT gen_random_uuid(),
  "aggregate_id" UUID NOT NULL,
  "type" VARCHAR(100) NOT NULL,
  "payload" JSONB NOT NULL,
  "attempts" INTEGER NOT NULL DEFAULT 0,
  "available_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  "published_at" TIMESTAMP(3),
  "created_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT "outbox_events_pkey" PRIMARY KEY ("id")
);

CREATE TABLE "processed_events" (
  "event_id" VARCHAR(128) NOT NULL,
  "event_type" VARCHAR(100) NOT NULL,
  "processed_at" TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT "processed_events_pkey" PRIMARY KEY ("event_id")
);

CREATE UNIQUE INDEX "orders_tracking_id_key" ON "orders"("tracking_id");
CREATE UNIQUE INDEX "orders_idempotency_key_key" ON "orders"("idempotency_key");
CREATE INDEX "orders_user_id_created_at_idx" ON "orders"("user_id", "created_at" DESC);
CREATE UNIQUE INDEX "order_items_order_id_product_id_key" ON "order_items"("order_id", "product_id");
CREATE INDEX "order_items_order_id_idx" ON "order_items"("order_id");
CREATE INDEX "order_history_order_id_created_at_idx" ON "order_history"("order_id", "created_at");
CREATE INDEX "outbox_events_published_at_available_at_idx" ON "outbox_events"("published_at", "available_at");

ALTER TABLE "order_items" ADD CONSTRAINT "order_items_order_id_fkey"
  FOREIGN KEY ("order_id") REFERENCES "orders"("id") ON DELETE CASCADE ON UPDATE CASCADE;
ALTER TABLE "order_history" ADD CONSTRAINT "order_history_order_id_fkey"
  FOREIGN KEY ("order_id") REFERENCES "orders"("id") ON DELETE CASCADE ON UPDATE CASCADE;
