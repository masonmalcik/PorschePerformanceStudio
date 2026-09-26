# Porsche Performance Studio (PPS)

Polyglot, serverless microservices intended for AWS Lambda.

## Services

| Service | Stack | Status |
| --- | --- | --- |
| `PPS_CatalogService` | Rust + MongoDB Atlas | Initial scaffold |
| `PPS_AuthService` | Go + Cognito + DynamoDB | Implemented and deployed |
| `PPS_CartService` | Node.js + DynamoDB | Planned |
| `PPS_OrderService` | C#/.NET + SQL Server | Planned |
| `PPS_PaymentService` | Node.js + DynamoDB | Planned |
| `PPS_NotificationService` | TypeScript/Node.js | Planned |
| `PPS_InventoryService` | Python + PostgreSQL | Planned |

Each service owns its runtime, dependencies, deployment definition, and data. Shared contracts can be added under `packages/contracts` once service-to-service APIs are introduced.
