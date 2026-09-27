import { randomUUID } from "node:crypto";
import type { EventEnvelope, RoutingKey } from "@pps/messaging";
import type { SQSEvent } from "aws-lambda";
import { createRuntime } from "./runtime.js";

const runtime = createRuntime();
if ("subscribe" in runtime.transport) {
  const subscriptions: Array<[string, RoutingKey]> = [
    ["ecom.order.inventory-reserved", "inventory.reserved"],
    ["ecom.order.inventory-rejected", "inventory.rejected"],
    ["ecom.order.inventory-released", "inventory.released"],
    ["ecom.order.payment-succeeded", "payment.succeeded"],
    ["ecom.order.payment-failed", "payment.failed"],
  ];
  for (const [queue, key] of subscriptions) {
    await runtime.transport.subscribe(queue, key, async (event) => {
      const result = await runtime.sqsHandler(asSqs(event));
      if (result.batchItemFailures.length) throw new Error(`Order event ${event.eventId} failed`);
    });
  }
}
runtime.outbox.start();
const server = runtime.app.listen(runtime.config.PORT, () => runtime.logger.info({ port: runtime.config.PORT }, "Order service listening"));
for (const signal of ["SIGINT", "SIGTERM"] as const) process.once(signal, () => void (async () => {
  runtime.outbox.stop(); server.close(); await runtime.transport.close(); await runtime.prisma.$disconnect();
})());

function asSqs(event: EventEnvelope): SQSEvent {
  return { Records: [{ messageId: randomUUID(), body: JSON.stringify(event) }] } as SQSEvent;
}
