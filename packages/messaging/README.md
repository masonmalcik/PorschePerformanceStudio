# PPS Messaging

Shared, strongly typed checkout-event contracts with two transport adapters:

- `RabbitMQService` uses `amqp-connection-manager` and confirm channels for local Docker development.
- `AwsMessagingService` publishes through SNS, while `createSqsBatchHandler` provides partial-batch failure handling for Lambda consumers.

Use `MESSAGE_TRANSPORT=rabbitmq` locally and `MESSAGE_TRANSPORT=aws` in deployed functions. Event names and payloads remain identical across transports.

The target saga sequence is `order.created` -> inventory decision -> `payment.requested` -> payment result. Inventory and Notification each receive filtered copies through their own queues, so one unavailable consumer cannot block another.

## Topology

| Resource | Type/purpose |
| --- | --- |
| `ecom.events.topic` | Durable topic exchange for domain events |
| `ecom.retry.exchange` | Durable direct exchange feeding bounded-delay retry queues |
| `ecom.deadletter.exchange` | Durable topic exchange for terminal failures |
| `ecom.payment.payment-requested` | Payment consumer for `payment.requested` |
| `ecom.order.payment-succeeded` | Order consumer for `payment.succeeded` |
| `ecom.order.payment-failed` | Order consumer for `payment.failed` |
| `<queue>.retry` | Durable TTL queue that dead-letters back to the event exchange |
| `<queue>.dlq` | Durable audit queue for malformed or exhausted events |

All publications are persistent and await publisher confirmation. Consumers use `noAck: false`. Successful handlers ACK; transient failures enter the bounded retry path; validation failures, `NonRetryableMessageError`, and exhausted retries use `nack(message, false, false)` and route through the queue's DLX.

RabbitMQ guarantees at-least-once delivery, not exactly-once processing. Application handlers must deduplicate `eventId` transactionally, as the Order and Payment inbox tables already do.

## Local broker

```powershell
docker compose up -d --wait
$env:RABBITMQ_URL = "amqp://pps:pps-local-only@localhost:5672"
npm run topology:init
```

The local management UI is available at `http://localhost:15672` using `pps` / `pps-local-only`.

## Publishing

```typescript
const messaging = new RabbitMQService({ urls: [process.env.RABBITMQ_URL!] });

await messaging.publish("order.created", {
  orderId,
  userId,
  totalAmount: "149.99",
  items: [{ productId, quantity: 1, pricePerUnit: "149.99" }],
});
```

## Consuming

```typescript
await messaging.subscribe(
  "ecom.order.payment-succeeded",
  "payment.succeeded",
  async (event) => {
    await orderSaga.paymentProcessed({
      eventId: event.eventId,
      type: "PaymentProcessed",
      aggregateId: event.payload.orderId,
      occurredAt: event.occurredAt,
      data: {
        orderId: event.payload.orderId,
        paymentId: event.payload.paymentIntentId,
      },
    });
  },
);
```

Keep each routing key on its own queue when handlers differ. Multiple service replicas may consume the same queue for competing-consumer load balancing.

## Production configuration

- Use `amqps://` and broker-issued credentials from a secret manager.
- Use a least-privilege virtual host rather than `/`.
- Deploy a three-node quorum-capable RabbitMQ cluster or a managed broker.
- Monitor connection churn, unacked messages, retry depth, DLQ depth, publish-confirm latency, and disk alarms.
- Apply broker policies for queue length and retention appropriate to your recovery objectives.

## Verification

```powershell
npm run typecheck
npm test
npm run build
```

Live integration tests run when `RABBITMQ_TEST_URL` is set. They publish through a real broker and assert both handler delivery and DLQ routing.
