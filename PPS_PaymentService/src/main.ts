import { PrismaClient } from "@prisma/client";
import pino from "pino";
import { createApp } from "./app.js";
import { loadConfig } from "./config.js";
import { PrismaPaymentRepository } from "./infrastructure/prisma-payment-repository.js";
import { OfficialStripeGateway } from "./infrastructure/stripe-gateway.js";
import { PaymentService } from "./services/payment-service.js";

const config = loadConfig();
const logger = pino({ level: config.LOG_LEVEL, redact: ["req.headers.authorization", "req.headers.stripe-signature"] });
const prisma = new PrismaClient();
const repository = new PrismaPaymentRepository(prisma);
const stripe = new OfficialStripeGateway(config.STRIPE_SECRET_KEY, config.STRIPE_WEBHOOK_SECRET);
const service = new PaymentService(repository, stripe, logger);
const server = createApp({ service, stripe, logger, trustedUserHeader: config.TRUSTED_USER_HEADER, corsOrigin: config.CORS_ORIGIN })
  .listen(config.PORT, () => logger.info({ port: config.PORT }, "Payment service listening"));

for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.once(signal, () => void (async () => {
    logger.info({ signal }, "Shutting down payment service");
    server.close();
    await prisma.$disconnect();
  })());
}
