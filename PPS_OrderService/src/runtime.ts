import { AwsMessagingService, RabbitMQService, createSqsRouter, type EventEnvelope } from "@pps/messaging";
import { PrismaClient } from "@prisma/client";
import pino from "pino";
import { createApp } from "./app.js";
import { loadConfig } from "./config.js";
import type { DomainEvent } from "./domain/order.js";
import { PrismaOrderRepository } from "./infrastructure/prisma-order-repository.js";
import { CheckoutPublisher } from "./messaging/checkout-publisher.js";
import { OutboxWorker } from "./messaging/outbox-worker.js";
import { OrderSaga } from "./services/order-saga.js";
import { OrderService } from "./services/order-service.js";

export function createRuntime() {
  const config = loadConfig();
  const logger = pino({ level: config.LOG_LEVEL, redact: ["req.headers.authorization"] });
  const prisma = new PrismaClient();
  const repository = new PrismaOrderRepository(prisma);
  const saga = new OrderSaga(repository);
  const transport = process.env.MESSAGE_TRANSPORT === "aws"
    ? new AwsMessagingService({ topicArn: env("EVENTS_TOPIC_ARN") })
    : new RabbitMQService({ urls: [process.env.RABBITMQ_URL ?? "amqp://pps:pps-local-only@localhost:5672"] });
  const outbox = new OutboxWorker(repository, new CheckoutPublisher(transport), logger, config.OUTBOX_POLL_INTERVAL_MS, config.OUTBOX_BATCH_SIZE);
  const app = createApp({ service: new OrderService(repository), logger, trustedUserHeader: config.TRUSTED_USER_HEADER, corsOrigin: config.CORS_ORIGIN });
  const sqsHandler = createSqsRouter({
    "inventory.reserved": (event) => saga.inventoryReserved(domain(event, "InventoryReserved")).then(noop),
    "inventory.rejected": (event) => saga.inventoryUnavailable(domain(event, "InventoryUnavailable")).then(noop),
    "inventory.released": (event) => saga.inventoryReleased(domain(event, "InventoryReleased")).then(noop),
    "payment.succeeded": (event) => saga.paymentProcessed(domain(event, "PaymentProcessed", { paymentId: event.payload.paymentIntentId })).then(noop),
    "payment.failed": (event) => saga.paymentFailed(domain(event, "PaymentFailed")).then(noop),
  }, logger);
  return { app, config, logger, prisma, outbox, sqsHandler, transport };
}
function domain(event: EventEnvelope, type: string, extra: Record<string, unknown> = {}): DomainEvent { const payload = event.payload as Record<string, unknown>; return { eventId: event.eventId, type, aggregateId: String(payload.orderId), occurredAt: event.occurredAt, data: { ...payload, ...extra } }; }
function env(name: string): string { const value = process.env[name]; if (!value) throw new Error(`${name} is required`); return value; }
function noop(): void {}
