# Code-first database policy

The Node.js/PostgreSQL services use Prisma ORM. Their `schema.prisma` files are the single source of truth, and Prisma-generated migration directories are immutable deployment artifacts committed with the application change.

| Service | Code-first schema | Generated migrations |
| --- | --- | --- |
| Order | `PPS_OrderService/prisma/schema.prisma` | `PPS_OrderService/prisma/migrations` |
| Payment | `PPS_PaymentService/prisma/schema.prisma` | `PPS_PaymentService/prisma/migrations` |

The Cart service uses DynamoDB and declares its table code-first in `PPS_CartService/template.yaml`; CloudFormation owns that table's lifecycle. The Auth and Catalog services retain their language-native persistence tooling because they are not Node.js/PostgreSQL services.

## Required change sequence

1. Change `schema.prisma`, repository types, and tests together.
2. Run `npm run db:migrate:create -- --name meaningful_name` against a disposable development database.
3. Review the generated migration for destructive operations.
4. Run integration tests against an isolated `TEST_DATABASE_URL`.
5. Commit the schema and generated migration in the same change.
6. Run `npm run db:migrate:deploy` from the deployment pipeline, never by generating migrations in production.

Production migrations use `migrate deploy`, which applies only checked-in pending migrations and does not use a shadow database. Development migration generation uses `migrate dev` and therefore needs a disposable development database plus shadow-database privileges or a separately configured shadow database.
