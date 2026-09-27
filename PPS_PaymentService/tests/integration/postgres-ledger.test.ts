import { randomUUID } from "node:crypto";
import { PrismaClient } from "@prisma/client";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { PrismaPaymentRepository } from "../../src/infrastructure/prisma-payment-repository.js";

const testDatabaseUrl = process.env.TEST_DATABASE_URL;

describe.runIf(Boolean(testDatabaseUrl))("PostgreSQL payment ledger", () => {
  let prisma: PrismaClient;
  const orderId = randomUUID();
  const eventId = `evt_integration_${randomUUID()}`;

  beforeAll(async () => {
    prisma = new PrismaClient({ datasources: { db: { url: testDatabaseUrl as string } } });
    await prisma.$connect();
  });

  afterAll(async () => {
    await prisma.paymentOutboxEvent.deleteMany({ where: { aggregateId: orderId } });
    await prisma.stripeWebhookEvent.deleteMany({ where: { eventId: { startsWith: "evt_integration_" } } });
    await prisma.paymentTransaction.deleteMany({ where: { orderId } });
    await prisma.$disconnect();
  });

  it("atomically deduplicates a webhook and its external outbox event", async () => {
    const repository = new PrismaPaymentRepository(prisma);
    const transaction = await repository.savePending({
      orderId,
      userId: "integration-user",
      amount: 14999,
      currency: "usd",
      idempotencyKey: `integration-${randomUUID()}`,
    }, `pi_integration_${randomUUID()}`);
    const webhook = {
      eventId,
      eventType: "payment_intent.succeeded",
      eventCreated: Math.floor(Date.now() / 1000),
      stripeIntentId: transaction.stripePaymentIntentId,
      status: "SUCCEEDED" as const,
      errorMessage: null,
    };
    const first = await repository.applyWebhook(webhook);
    const replay = await repository.applyWebhook(webhook);
    expect(first.duplicate).toBe(false);
    expect(replay.duplicate).toBe(true);
    await expect(prisma.paymentOutboxEvent.count({ where: { aggregateId: orderId } })).resolves.toBe(1);
    await expect(prisma.stripeWebhookEvent.count({ where: { eventId } })).resolves.toBe(1);
  });
});
