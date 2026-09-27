import { z } from "zod";

const money = z.string().regex(/^\d{1,12}(?:\.\d{2})$/);
const orderItem = z.object({ productId: z.string().min(1).max(128), quantity: z.number().int().positive(), pricePerUnit: money }).strict();

export const eventSchemas = {
  "order.created": z.object({
    orderId: z.string().uuid(), userId: z.string().min(1).max(128), totalAmount: money,
    items: z.array(orderItem).min(1),
  }).strict(),
  "payment.requested": z.object({
    orderId: z.string().uuid(), userId: z.string().min(1).max(128), totalAmount: money,
    currency: z.string().length(3).transform((value) => value.toLowerCase()),
  }).strict(),
  "payment.succeeded": z.object({
    paymentIntentId: z.string().min(1).max(255), orderId: z.string().uuid(), amount: z.number().int().positive(),
  }).strict(),
  "payment.failed": z.object({
    paymentIntentId: z.string().min(1).max(255), orderId: z.string().uuid(), amount: z.number().int().positive(),
    reason: z.string().max(1000).optional(),
  }).strict(),
  "inventory.reserved": z.object({
    orderId: z.string().uuid(), reservationId: z.string().min(1).max(255),
  }).strict(),
  "inventory.rejected": z.object({
    orderId: z.string().uuid(), reason: z.string().min(1).max(1000),
  }).strict(),
  "inventory.released": z.object({
    orderId: z.string().uuid(), reservationId: z.string().min(1).max(255),
  }).strict(),
} as const;

export type RoutingKey = keyof typeof eventSchemas;
export type EventPayloads = { [K in RoutingKey]: z.infer<(typeof eventSchemas)[K]> };

export interface EventEnvelope<K extends RoutingKey = RoutingKey> {
  eventId: string;
  type: K;
  version: 1;
  occurredAt: string;
  correlationId?: string;
  payload: EventPayloads[K];
}

export const envelopeSchema = z.object({
  eventId: z.string().uuid(), type: z.string(), version: z.literal(1), occurredAt: z.iso.datetime(),
  correlationId: z.string().max(128).optional(), payload: z.unknown(),
}).strict();
