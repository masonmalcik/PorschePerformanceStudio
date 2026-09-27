# PPS Cart Service

Express/TypeScript cart service using DynamoDB as its durable store and fail-open Memcached cache-aside behavior.

## Infrastructure

The included `template.yaml` creates one DynamoDB Standard table with 5 provisioned RCUs and 5 provisioned WCUs, server-side encryption, native TTL, and retention protection. The table stores one document per user cart. Optimistic version checks prevent concurrent updates from silently overwriting each other.

Deploy the development table with the AWS profile already configured for PPS:

```powershell
sam validate --lint
sam deploy --guided
```

Recommended guided-deployment values:

- Stack name: `pps-cart-dev`
- Region: `us-east-1`
- Parameter `Environment`: `dev`
- Save arguments to configuration: `Y`

Copy the safe example configuration and start local Memcached:

```powershell
Copy-Item .env.example .env
docker compose up -d
npm install
npm run dev
```

The AWS SDK uses the normal credential provider chain, including `AWS_PROFILE`, AWS SSO, environment credentials, or a container/task role. Do not put access keys in `.env`.

## API

The service listens on port `3004` by default. The API gateway/Auth service must strip any client-supplied `x-authenticated-user-id` header and replace it with the verified Cognito subject.

- `GET /health`
- `GET /cart/:userId`
- `POST /cart/:userId/items` with `{ "itemId": "sku", "quantity": 2, "unitPrice": "149.99" }`
- `DELETE /cart/:userId/items/:itemId`
- `DELETE /cart/:userId`

## Cache and abandonment

Memcached entries use `cart:{userId}` and expire after 24 hours. DynamoDB is always authoritative: reads fail open on cache errors, and mutations commit to DynamoDB before refreshing or invalidating cache.

Each cart document has a numeric `ttl` attribute set 24 hours after its latest mutation. DynamoDB TTL removes abandoned carts automatically without consuming write capacity. TTL deletion is asynchronous.

## Free Tier safeguards

- Standard table class, not Standard-IA.
- Provisioned capacity fixed at 5 RCUs and 5 WCUs; no auto scaling.
- No global tables, streams, backups, or paid point-in-time recovery.
- Native TTL cleanup.
- `DeletionPolicy` and `UpdateReplacePolicy` are `Retain`.

AWS Free Tier capacity is shared per payer account and Region. This template cannot guarantee a zero bill if other tables bring the account total above the allowance, storage exceeds the allowance, paid features are enabled, or network transfer is chargeable. Configure an AWS Budget alert as an account-level guardrail.

## Verification

```powershell
npm run typecheck
npm test
npm run build
```

Tests use deterministic in-memory database/cache adapters and never access an AWS account.
