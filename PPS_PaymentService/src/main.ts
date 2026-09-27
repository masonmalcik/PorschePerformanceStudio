import { randomUUID } from "node:crypto";
import type { EventEnvelope } from "@pps/messaging";
import type { SQSEvent } from "aws-lambda";
import { createRuntime } from "./runtime.js";
const runtime = createRuntime();
if ("subscribe" in runtime.transport) await runtime.transport.subscribe("ecom.payment.payment-requested", "payment.requested", async (event) => { const result = await runtime.sqsHandler(asSqs(event)); if (result.batchItemFailures.length) throw new Error(`Payment event ${event.eventId} failed`); });
runtime.outbox.start();
const server = runtime.app.listen(runtime.config.PORT, () => runtime.logger.info({ port: runtime.config.PORT }, "Payment service listening"));
for (const signal of ["SIGINT", "SIGTERM"] as const) process.once(signal, () => void (async () => { runtime.outbox.stop(); server.close(); await runtime.transport.close(); await runtime.prisma.$disconnect(); })());
function asSqs(event: EventEnvelope): SQSEvent { return { Records: [{ messageId: randomUUID(), body: JSON.stringify(event) }] } as SQSEvent; }
