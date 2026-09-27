import { randomUUID } from "node:crypto";
import { PrismaClient } from "@prisma/client";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { PrismaOrderRepository } from "../../src/infrastructure/prisma-order-repository.js";

const databaseUrl = process.env.TEST_DATABASE_URL;

describe.runIf(Boolean(databaseUrl))("PostgreSQL relational integrity", () => {
  let prisma: PrismaClient;

  beforeAll(async () => {
    prisma = new PrismaClient({ datasources: { db: { url: databaseUrl as string } } });
    await prisma.$connect();
  });

  afterAll(async () => {
    await prisma.order.deleteMany({ where: { idempotencyKey: { startsWith: "integration-" } } });
    await prisma.$disconnect();
  });

  it("rejects an OrderItem without a parent Order", async () => {
    await expect(prisma.orderItem.create({
      data: { orderId: randomUUID(), productId: "orphan-product", quantity: 1, pricePerUnit: "10.00" },
    })).rejects.toMatchObject({ code: "P2003" });
  });

  it("rolls back the complete order transaction when nested item creation fails", async () => {
    const idempotencyKey = `integration-rollback-${randomUUID()}`;
    const repository = new PrismaOrderRepository(prisma);
    await expect(repository.create({
      userId: "integration-user",
      idempotencyKey,
      items: [
        { productId: "duplicate-product", quantity: 1, pricePerUnit: "10.00" },
        { productId: "duplicate-product", quantity: 1, pricePerUnit: "10.00" },
      ],
    })).rejects.toBeDefined();
    await expect(prisma.order.count({ where: { idempotencyKey } })).resolves.toBe(0);
  });

  it("replays the same order for a repeated idempotency key", async () => {
    const idempotencyKey = `integration-idempotent-${randomUUID()}`;
    const repository = new PrismaOrderRepository(prisma);
    const input = {
      userId: "integration-user",
      idempotencyKey,
      items: [{ productId: "product-1", quantity: 2, pricePerUnit: "10.00" }],
    };
    const first = await repository.create(input);
    const second = await repository.create(input);
    expect(first.replayed).toBe(false);
    expect(second.replayed).toBe(true);
    expect(second.order.id).toBe(first.order.id);
  });
});
