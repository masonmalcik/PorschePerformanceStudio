# Porsche Performance Studio (PPS)

Polyglot, serverless microservices intended for AWS Lambda.

## Services

| Service | Stack | Status |
| --- | --- | --- |
| `PPS_CatalogService` | Rust + MongoDB Atlas | Initial scaffold |
| `PPS_AuthService` | Go + Cognito + DynamoDB | Implemented and deployed |
| `PPS_CartService` | Node.js/TypeScript + DynamoDB + Memcached | Implemented |
| `PPS_OrderService` | Node.js/TypeScript + PostgreSQL + transactional outbox | Implemented |
| `PPS_PaymentService` | Node.js/TypeScript + Stripe + PostgreSQL ledger | Implemented |
| `PPS_NotificationService` | TypeScript/Node.js | Planned |
| `PPS_InventoryService` | Python + PostgreSQL | Planned |

Each service owns its runtime, dependencies, deployment definition, and data. Shared contracts can be added under `packages/contracts` once service-to-service APIs are introduced.

Shared RabbitMQ event contracts, retry/DLQ topology, and the reconnecting confirm-channel client live in `packages/messaging`.
