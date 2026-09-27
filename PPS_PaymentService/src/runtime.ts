import { AwsMessagingService, RabbitMQService, createSqsBatchHandler } from "@pps/messaging";
import { PrismaClient } from "@prisma/client";
import pino from "pino";
import { createApp } from "./app.js";
import { loadConfig } from "./config.js";
import { PrismaPaymentRepository } from "./infrastructure/prisma-payment-repository.js";
import { OfficialStripeGateway } from "./infrastructure/stripe-gateway.js";
import { PaymentOutboxWorker } from "./messaging/payment-outbox-worker.js";
import { PaymentService } from "./services/payment-service.js";
export function createRuntime() {
  const config = loadConfig(); const logger = pino({ level: config.LOG_LEVEL, redact: ["req.headers.authorization", "req.headers.stripe-signature"] }); const prisma = new PrismaClient(); const repository = new PrismaPaymentRepository(prisma); const stripe = new OfficialStripeGateway(config.STRIPE_SECRET_KEY, config.STRIPE_WEBHOOK_SECRET); const service = new PaymentService(repository, stripe, logger);
  const transport = process.env.MESSAGE_TRANSPORT === "aws" ? new AwsMessagingService({ topicArn: env("EVENTS_TOPIC_ARN") }) : new RabbitMQService({ urls: [process.env.RABBITMQ_URL ?? "amqp://pps:pps-local-only@localhost:5672"] });
  const outbox = new PaymentOutboxWorker(repository, transport, logger); const app = createApp({ service, stripe, logger, trustedUserHeader: config.TRUSTED_USER_HEADER, corsOrigin: config.CORS_ORIGIN });
  const sqsHandler = createSqsBatchHandler("payment.requested", async (event) => { await service.createIntent({ orderId: event.payload.orderId, userId: event.payload.userId, amount: cents(event.payload.totalAmount), currency: event.payload.currency, idempotencyKey: event.eventId }); }, logger);
  return { app, config, logger, prisma, transport, outbox, sqsHandler };
}
function cents(value: string): number { const amount = Number(value); if (!Number.isFinite(amount)) throw new Error("Invalid payment amount"); return Math.round(amount * 100); }
function env(name: string): string { const value = process.env[name]; if (!value) throw new Error(`${name} is required`); return value; }
