# Porsche Performance Studio (PPS)

Polyglot, serverless microservices intended for AWS Lambda.

## Services

| Service | Stack | Status |
| --- | --- | --- |
| `PPS_CatalogService` | Rust + MongoDB Atlas | Implemented and deployed |
| `PPS_AuthService` | Go + Cognito + DynamoDB | Implemented and deployed |
| `PPS_CartService` | Node.js/TypeScript + DynamoDB + Memcached | Implemented and deployed |
| `PPS_OrderService` | Node.js/TypeScript + PostgreSQL + transactional outbox | Implemented and deployed |
| `PPS_PaymentService` | Node.js/TypeScript + Stripe + PostgreSQL ledger | Implemented and deployed |
| `PPS_NotificationService` | TypeScript/Node.js | Planned |
| `PPS_InventoryService` | Python + PostgreSQL | Planned |

Each service owns its runtime, dependencies, deployment definition, and data. The unified AWS HTTP API is available at `https://bjq3n8ojr9.execute-api.us-east-1.amazonaws.com`.

Shared event contracts, the local RabbitMQ adapter, and the production SNS/SQS adapter live in `packages/messaging`. AWS deployment and packaging instructions live in `deploy/README.md`.
