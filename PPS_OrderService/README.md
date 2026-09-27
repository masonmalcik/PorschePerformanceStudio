# PPS Order Service

Express/TypeScript order service backed by PostgreSQL and Prisma. Checkout uses an orchestrated saga, transactional outbox, optimistic order versions, idempotent checkout requests, and an inbox table for deduplicating consumed events.

## Setup

Copy `.env.example` to the ignored `.env` file and place the Aiven PostgreSQL connection string in `DATABASE_URL`. The service never requires that credential in source control.

```powershell
npm install
npm run prisma:generate
npm run prisma:migrate:deploy
npm run dev
```

## Code-first migration workflow

`prisma/schema.prisma` is the authoritative model for tables, columns, native database types, relations, uniqueness rules, and indexes. Files under `prisma/migrations` are generated deployment artifacts and must be committed; do not create tables manually in Aiven.

After editing the Prisma schema, generate a reviewable migration against a disposable development database:

```powershell
npm run db:migrate:create -- --name describe_the_change
```

Review the generated migration, then apply it to the development database and regenerate the typed client:

```powershell
npm run db:migrate:dev
npm run db:generate
```

Apply already-reviewed migrations to staging or production from CI/CD:

```powershell
npm run db:migrate:status
npm run db:migrate:deploy
```

Never run `migrate dev`, `db push`, or `migrate reset` against production. Prisma `migrate dev` requires a separate shadow database when the development account cannot create databases.

For hosted PostgreSQL, URL-encode credentials and use SSL plus bounded application pooling:

```dotenv
DATABASE_URL="postgresql://user:URL_ENCODED_PASSWORD@host:22478/defaultdb?sslmode=require&connection_limit=5&pool_timeout=10"
```

The API listens on port `3005` by default. The API gateway must remove client-supplied `x-authenticated-user-id` values and inject the verified Cognito subject.

## API

- `POST /orders`, with `idempotency_key` and `x-authenticated-user-id` headers
- `GET /orders/:id`
- `GET /orders/user/:userId`
- `PATCH /orders/:id/cancel`

Example body:

```json
{
  "items": [
    { "productId": "product-123", "quantity": 2, "pricePerUnit": "149.99" }
  ]
}
```

Money is represented as decimal strings at API boundaries and `DECIMAL(14,2)` in PostgreSQL.

## Saga

| Incoming event/action | Current state | New state | Outbound events |
| --- | --- | --- | --- |
| Create order | — | `PENDING` | `OrderCreated` |
| `InventoryReserved` | `PENDING` | `STOCK_RESERVED` | `PaymentRequested` |
| `PaymentProcessed` | `STOCK_RESERVED` | `PAID`, then `CONFIRMED` | `OrderPaid`, `OrderConfirmed` |
| `InventoryUnavailable` | `PENDING` | `FAILED` | `OrderFailed` |
| `PaymentFailed` | `STOCK_RESERVED` | `FAILED` | `InventoryReleaseRequested`, `OrderFailed` |
| `InventoryReleased` | `FAILED`/`CANCELLED` | `COMPENSATED` | `OrderCompensated` |

Order mutations and outbound events share one PostgreSQL transaction. The outbox worker publishes asynchronously and retries broker failures. `MessagePublisher` and `MessageConsumer` are stable transport interfaces; the included in-memory broker is development-only. Replace it with RabbitMQ, Kafka, or SNS/SQS before running multiple production replicas.

Consumers should assume at-least-once delivery. `ProcessedEvent` deduplicates inbound events, while downstream consumers must similarly deduplicate outbound `eventId` values.

## Tests

```powershell
npm run typecheck
npm test
npm run build
```

Database integration tests are safely skipped unless `TEST_DATABASE_URL` points to a disposable PostgreSQL database. Test startup applies all migrations automatically before executing tests:

```powershell
$env:TEST_DATABASE_URL = "postgresql://..."
npm run test:integration
```

Never point destructive integration tests at production or a shared Aiven database.
