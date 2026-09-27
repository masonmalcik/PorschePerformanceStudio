# PPS Payment Service

Express/TypeScript payment service using Stripe Payment Intents and PostgreSQL/Prisma as a local immutable processing ledger.

## Security model

- Stripe secret and webhook keys are environment-only secrets.
- The webhook route receives `express.raw()` bytes before JSON middleware and verifies `stripe-signature` with `stripe.webhooks.constructEvent`.
- Amounts are positive integer minor units (for example, `$149.99 USD` is `14999`), never floating-point values.
- Currency codes are normalized to lowercase three-letter codes.
- Payment creation requires the trusted Cognito user header and an `Idempotency-Key` header.
- Stripe receives the same idempotency key and the order ID in PaymentIntent metadata.
- Stripe webhook IDs are unique in `stripe_webhook_events`, preventing duplicate ledger transitions and duplicate outbox rows.
- A serializable database transaction updates the ledger, records the webhook, and appends `PaymentProcessed` or `PaymentFailed` to the transactional outbox atomically.
- A successful payment is terminal and cannot be downgraded by a late failure event.

The gateway must remove client-supplied `x-authenticated-user-id` values and replace them with the verified Cognito subject. The Order service should remain authoritative for the amount: before production, the payment request must originate from, or be verified against, the trusted Order service rather than accepting a browser-calculated total.

## Setup

Copy `.env.example` to `.env`, then provide a rotated Aiven URL and Stripe test-mode secrets. Never commit `.env`.

```powershell
npm install
npm run prisma:generate
npm run prisma:migrate:deploy
npm run dev
```

## Code-first migration workflow

`prisma/schema.prisma` is the authoritative model for tables, columns, native types, uniqueness rules, and indexes. The SQL snapshots under `prisma/migrations` are Prisma-generated, reviewed deployment artifacts—not a second manually maintained schema.

Generate a migration after changing the Prisma schema:

```powershell
npm run db:migrate:create -- --name describe_the_change
```

Apply it to development and regenerate the typed client:

```powershell
npm run db:migrate:dev
npm run db:generate
```

Apply checked-in migrations to staging or production from CI/CD:

```powershell
npm run db:migrate:status
npm run db:migrate:deploy
```

Never use `migrate dev`, `db push`, or `migrate reset` against production. Hosted PostgreSQL URLs must URL-encode credentials and include SSL and bounded pooling:

```dotenv
DATABASE_URL="postgresql://user:URL_ENCODED_PASSWORD@host:22478/defaultdb?sslmode=require&connection_limit=5&pool_timeout=10"
```

The API listens on port `3006` by default.

## Endpoints

### `POST /payments/create-intent`

Headers:

```text
x-authenticated-user-id: cognito-sub
Idempotency-Key: checkout-attempt-id
```

Body:

```json
{
  "orderId": "11111111-1111-4111-8111-111111111111",
  "amount": 14999,
  "currency": "usd"
}
```

### `POST /payments/webhook`

Configure Stripe to send events to this route. Subscribe to:

- `payment_intent.succeeded`
- `payment_intent.payment_failed`

For local Stripe CLI testing:

```powershell
stripe listen --forward-to localhost:3006/payments/webhook
```

Use the displayed `whsec_...` value as `STRIPE_WEBHOOK_SECRET`.

## Event delivery

Accepted status changes create `PaymentProcessed` or `PaymentFailed` rows in `payment_outbox_events`. Connect an outbox relay implementing the same durable broker contract used by the Order service. Broker delivery is inherently at-least-once; consumers must deduplicate by outbox event ID. The database guarantees that a duplicate Stripe webhook does not create a second outbox event.

## Verification

```powershell
npm run typecheck
npm test
npm run build
npm audit --omit=dev
```

The tests use Stripe's official test-signature generator and do not call Stripe or Aiven.

When `TEST_DATABASE_URL` is set, Vitest first runs `prisma migrate deploy` against that isolated database and then executes the PostgreSQL ledger integration suite. Startup aborts if `TEST_DATABASE_URL` equals `DATABASE_URL`.
