import { PrismaClient } from "@prisma/client";
import pino from "pino";
import { createApp } from "./app.js";
import { loadConfig } from "./config.js";
import { PrismaOrderRepository } from "./infrastructure/prisma-order-repository.js";
import { InMemoryBroker } from "./messaging/in-memory-broker.js";
import { OutboxWorker } from "./messaging/outbox-worker.js";
import { OrderSaga } from "./services/order-saga.js";
import { OrderService } from "./services/order-service.js";

const config = loadConfig();
const logger = pino({ level: config.LOG_LEVEL, redact: ["req.headers.authorization"] });
const prisma = new PrismaClient();
const repository = new PrismaOrderRepository(prisma);
const service = new OrderService(repository);
const saga = new OrderSaga(repository);

// Development adapter only. Replace with a durable broker adapter implementing
// MessagePublisher and MessageConsumer before deploying multiple replicas.
const broker = new InMemoryBroker();
await broker.subscribe("InventoryReserved", (event) => saga.inventoryReserved(event).then(() => undefined));
await broker.subscribe("InventoryUnavailable", (event) => saga.inventoryUnavailable(event).then(() => undefined));
await broker.subscribe("PaymentProcessed", (event) => saga.paymentProcessed(event).then(() => undefined));
await broker.subscribe("PaymentFailed", (event) => saga.paymentFailed(event).then(() => undefined));
await broker.subscribe("InventoryReleased", (event) => saga.inventoryReleased(event).then(() => undefined));

const outbox = new OutboxWorker(repository, broker, logger, config.OUTBOX_POLL_INTERVAL_MS, config.OUTBOX_BATCH_SIZE);
outbox.start();
const server = createApp({ service, logger, trustedUserHeader: config.TRUSTED_USER_HEADER, corsOrigin: config.CORS_ORIGIN })
  .listen(config.PORT, () => logger.info({ port: config.PORT }, "Order service listening"));

for (const signal of ["SIGINT", "SIGTERM"] as const) {
  process.once(signal, () => void (async () => {
    logger.info({ signal }, "Shutting down order service");
    outbox.stop();
    server.close();
    await prisma.$disconnect();
  })());
}
