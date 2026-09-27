# PPS Catalog Service

Rust AWS Lambda API backed by one MongoDB Atlas database. The service owns brands, categories, products, Porsche vehicle taxonomy/configuration, and product fitments.

Brand documents store a required `imageName` string. Corresponding files live under `assets/brands`; MongoDB never stores image bytes or a machine-specific path. The included placeholder should be replaced with approved brand artwork.

Future application imagery is organized under `assets/products`, `assets/Page-Backgrounds`, and `assets/vehicle-models`.

## N-tier architecture

- `presentation`: Lambda HTTP routing, request parsing, response and error mapping
- `application`: business services, use cases, repository ports, and image-store port
- `domain`: database-neutral entities, value objects, identifiers, and vehicle/component taxonomy
- `data`: MongoDB documents/conversions/repositories plus filesystem image-store adapter
- `seed`: compiled, idempotent reference-data definitions
- `bin/api.rs`: Lambda entry point
- `bin/migrate.rs`: explicit collection/validator/index migration runner
- `bin/seed.rs`: explicit code-first seed runner

Migrations and seeds never execute during API Lambda cold starts.

Dependencies flow inward: `presentation -> application -> domain`. The `data` tier implements application-owned ports; only `data` imports MongoDB or accesses asset folders. See `docs/architecture.md`.

## Database collections

`brands`, `categories`, `products`, `vehicle_models`, `vehicle_generations`, `vehicle_trims`, `vehicle_configurations`, `product_fitments`, `_catalog_migrations`, and `_catalog_seed_runs` all live in the `pps_catalog` database.

## API

- `GET /health`
- `GET /products` (public, filtered cursor page)
- `GET /products/{id}`
- `GET /api-docs/openapi.json`
- `POST /admin/products`
- `PATCH /admin/products/{id}`
- `DELETE /admin/products/{id}` (soft deletion)
- `POST /admin/vehicle-models`
- `GET /admin/vehicle-models`
- `POST /admin/vehicle-generations`
- `GET /admin/vehicle-generations`
- `POST /admin/vehicle-trims`
- `GET /admin/brands`
- `GET /admin/categories`
- `POST /admin/categories`

Public responses omit persistence metadata and inactive/version fields. Public product reads emit `ETag` plus `Cache-Control: public, max-age=60, s-maxage=300`, and honor `If-None-Match` with `304 Not Modified`.

## Product cache

Individual product reads use a Redis Cluster cache-aside decorator around the MongoDB repository. Keys use the cluster-safe `product:{id}` pattern and JSON values expire after one hour. Cache misses read MongoDB and backfill Redis; successful product updates and deactivations evict the corresponding key after the database commit.

Set `REDIS_CLUSTER_URLS` to a comma-separated list of `redis://` or TLS `rediss://` cluster nodes and optionally set `REDIS_POOL_SIZE` (default `16`). The async Redis connections are managed by a bounded `bb8` pool. Pool acquisition and cache commands have a 250 ms bound; connection errors, timeouts, invalid cached JSON, and startup configuration errors are logged and fail open to MongoDB. Leaving `REDIS_CLUSTER_URLS` empty disables the cache.

For AWS ElastiCache, deploy the Lambda in subnets and security groups that can reach the cluster, and use TLS endpoints. Network placement is intentionally infrastructure-specific and is not created by this service template.

`GET /products` accepts `pageSize` (1-100), `cursor`, `brandId`, `categoryId`, `saleType`, `minPrice`, `maxPrice`, `q`, `sort` (`name`, `newest`, or `price`), and `direction` (`asc` or `desc`). Unknown or duplicate parameters are rejected.

For local development only, set `ADMIN_AUTH_MODE=development`, choose an `ADMIN_DEV_TOKEN` of at least 24 characters, and send it as `x-pps-admin-key` on administrative requests. The default `disabled` mode denies every administrative request. The authorization port is ready for a future Cognito adapter.

HTTP activity and application counters are emitted as structured JSON logs. AWS can turn the `application_metric` records into CloudWatch metric filters; a future deployment can swap the metrics adapter for Embedded Metric Format without changing application code.

Create vehicle model request:

```json
{
  "name": "911",
  "modelCode": "911"
}
```

The service generates `_id`, `createdAt`, `updatedAt`, `isActive`, and `version`. Reusing an existing `modelCode` returns HTTP `409 Conflict`.

Creating a vehicle trim requires a generation and exactly one `vehicleModels` ID: the model associated with the generation.

```json
{
  "generationId": "68c8b9a713c53c1d8d950e21",
  "vehicleModels": ["68c8b9a713c53c1d8d950e20"],
  "name": "GT3",
  "trimCode": "GT3",
  "timeframe": { "startYear": 2022, "endYear": null }
}
```

Create an engine with `POST /admin/engines` using the local admin header. Submit one or more `vehicleTrims` IDs. Every selected trim must belong to the chosen model and generation; displacement is measured in liters. Factory codes are unique within each trim.

```json
{
  "vehicleModel": "68c8b9a713c53c1d8d950e20",
  "vehicleGeneration": "68c8b9a713c53c1d8d950e21",
  "vehicleTrims": ["68c8b9a713c53c1d8d950e22"],
  "alloyMaterial": "Aluminum",
  "factoryCode": "M97.76",
  "displacement": 3.6,
  "horsepower": 415,
  "rpm": 7600,
  "layout": "F6",
  "aspirationType": "naturally_aspirated",
  "fuelDelivery": "fuel_injected"
}
```

Prices are represented as JSON decimal strings in USD. The currency field is optional on writes and defaults to USD:

```json
{
  "sku": "PPS-911-GT3-001",
  "brandId": "68c8b9a713c53c1d8d950e20",
  "name": "Example Performance Package",
  "modelNumber": "PPS-GT3-001",
  "description": "Performance package for the 911 GT3.",
  "imageName": "example-performance-package.webp",
  "price": { "amount": "24999.00" },
  "saleType": "retail",
  "categoryIds": ["68c8b9a713c53c1d8d950e21"]
}
```

## Local setup

Copy `.env.example` to the ignored `.env` file and supply the Atlas URI. The API, migration, and seed binaries automatically load this file during local development; environment variables supplied by AWS still take precedence in deployment.

```powershell
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo run --bin migrate
cargo run --bin seed -- --profile minimal
```

Seed profiles are `minimal`, `development`, and `production`. They currently share the same approved reference taxonomy; environment-specific records can be added later without changing the runner.

The OpenAPI contract tests run with the normal test suite. The MongoDB integration test is intentionally ignored unless explicitly requested because it creates and drops an isolated temporary database:

```powershell
cargo test --test mongodb_integration -- --ignored
```

## AWS deployment

The SAM template builds the `api` binary for ARM64 on `provided.al2023`. Store a JSON secret such as `{ "MONGODB_URI": "..." }` in AWS Secrets Manager and pass its ARN as `MongoDbSecretArn`.

```powershell
cargo lambda build --release --arm64 --bin api
sam build
sam deploy --guided
```
