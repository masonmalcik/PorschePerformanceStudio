import { randomUUID } from "node:crypto";
import amqp from "amqplib";
import { afterAll, describe, expect, it } from "vitest";
import { NonRetryableMessageError, RabbitMQService } from "../../src/rabbitmq-service.js";

const url = process.env.RABBITMQ_TEST_URL;
const queues: string[] = [];
let service: RabbitMQService | undefined;

describe.runIf(Boolean(url))("RabbitMQ live integration", () => {
  afterAll(async () => {
    await service?.close();
    const connection = await amqp.connect(url as string);
    const channel = await connection.createChannel();
    for (const queue of queues) {
      await channel.deleteQueue(queue);
      await channel.deleteQueue(`${queue}.retry`);
      await channel.deleteQueue(`${queue}.dlq`);
    }
    await channel.close();
    await connection.close();
  });

  it("routes a confirmed publication to the designated handler", async () => {
    service = new RabbitMQService({ urls: [url as string], retryDelayMs: 100, maxRetries: 1 });
    const queue = `ecom.test.order-created.${randomUUID().replaceAll("-", "")}`;
    queues.push(queue);
    let resolveHandled: (() => void) | undefined;
    const handled = new Promise<void>((resolve) => { resolveHandled = resolve; });
    await service.subscribe(queue, "order.created", async () => { resolveHandled?.(); });
    await service.publish("order.created", {
      orderId: randomUUID(), userId: "integration-user", totalAmount: "20.00",
      items: [{ productId: "product-1", quantity: 2, pricePerUnit: "10.00" }],
    });
    await expect(Promise.race([handled, timeout(8000)])).resolves.toBeUndefined();
  }, 10_000);

  it("routes a permanently failed event to its DLQ", async () => {
    service ??= new RabbitMQService({ urls: [url as string], retryDelayMs: 100, maxRetries: 1 });
    const queue = `ecom.test.deadletter.${randomUUID().replaceAll("-", "")}`;
    queues.push(queue);
    await service.subscribe(queue, "order.created", async () => { throw new NonRetryableMessageError("invalid order"); });
    await service.publish("order.created", {
      orderId: randomUUID(), userId: "integration-user", totalAmount: "10.00",
      items: [{ productId: "product-1", quantity: 1, pricePerUnit: "10.00" }],
    });

    const connection = await amqp.connect(url as string);
    const channel = await connection.createChannel();
    try {
      let deadLetter = false;
      for (let attempt = 0; attempt < 25 && !deadLetter; attempt++) {
        deadLetter = Boolean(await channel.get(`${queue}.dlq`, { noAck: true }));
        if (!deadLetter) await new Promise((resolve) => setTimeout(resolve, 100));
      }
      expect(deadLetter).toBe(true);
    } finally {
      await channel.close();
      await connection.close();
    }
  });
});

function timeout(milliseconds: number): Promise<never> {
  return new Promise((_, reject) => setTimeout(() => reject(new Error("Timed out waiting for RabbitMQ event")), milliseconds));
}
