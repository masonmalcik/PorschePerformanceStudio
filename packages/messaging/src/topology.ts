import type { ConfirmChannel } from "amqplib";
import type { RoutingKey } from "./contracts.js";

export const EVENTS_EXCHANGE = "ecom.events.topic";
export const DEAD_LETTER_EXCHANGE = "ecom.deadletter.exchange";
export const RETRY_EXCHANGE = "ecom.retry.exchange";

export interface SubscriptionDefinition { queueName: string; routingKeys: RoutingKey[] }

export const checkoutTopology: SubscriptionDefinition[] = [
  { queueName: "ecom.inventory.order-created", routingKeys: ["order.created"] },
  { queueName: "ecom.payment.payment-requested", routingKeys: ["payment.requested"] },
  { queueName: "ecom.order.inventory-reserved", routingKeys: ["inventory.reserved"] },
  { queueName: "ecom.order.inventory-rejected", routingKeys: ["inventory.rejected"] },
  { queueName: "ecom.order.inventory-released", routingKeys: ["inventory.released"] },
  { queueName: "ecom.order.payment-succeeded", routingKeys: ["payment.succeeded"] },
  { queueName: "ecom.order.payment-failed", routingKeys: ["payment.failed"] },
  { queueName: "ecom.notification.payment-succeeded", routingKeys: ["payment.succeeded"] },
  { queueName: "ecom.notification.payment-failed", routingKeys: ["payment.failed"] },
  { queueName: "ecom.notification.inventory-rejected", routingKeys: ["inventory.rejected"] },
];

export async function assertBaseTopology(channel: ConfirmChannel): Promise<void> {
  await channel.assertExchange(EVENTS_EXCHANGE, "topic", { durable: true });
  await channel.assertExchange(DEAD_LETTER_EXCHANGE, "topic", { durable: true });
  await channel.assertExchange(RETRY_EXCHANGE, "direct", { durable: true });
}

export async function assertSubscriptionTopology(channel: ConfirmChannel, definition: SubscriptionDefinition, retryDelayMs: number): Promise<void> {
  validateQueueName(definition.queueName);
  if (definition.routingKeys.length !== 1) throw new Error("Each retryable queue must have exactly one routing key");
  const routingKey = definition.routingKeys[0];
  if (!routingKey) throw new Error("A routing key is required");
  await assertBaseTopology(channel);
  await channel.assertQueue(definition.queueName, {
    durable: true,
    arguments: { "x-dead-letter-exchange": DEAD_LETTER_EXCHANGE, "x-dead-letter-routing-key": definition.queueName },
  });
  await channel.bindQueue(definition.queueName, EVENTS_EXCHANGE, routingKey);
  const retryQueue = `${definition.queueName}.retry`;
  await channel.assertQueue(retryQueue, {
    durable: true,
    arguments: {
      "x-message-ttl": retryDelayMs,
      "x-dead-letter-exchange": EVENTS_EXCHANGE,
      "x-dead-letter-routing-key": routingKey,
    },
  });
  await channel.bindQueue(retryQueue, RETRY_EXCHANGE, definition.queueName);
  const dlq = `${definition.queueName}.dlq`;
  await channel.assertQueue(dlq, { durable: true });
  await channel.bindQueue(dlq, DEAD_LETTER_EXCHANGE, definition.queueName);
}

export async function assertCheckoutTopology(channel: ConfirmChannel, retryDelayMs = 5000): Promise<void> {
  for (const definition of checkoutTopology) await assertSubscriptionTopology(channel, definition, retryDelayMs);
}

function validateQueueName(queueName: string): void {
  if (!/^ecom\.[a-z0-9.-]{1,180}$/.test(queueName)) throw new Error(`Invalid queue name: ${queueName}`);
}
