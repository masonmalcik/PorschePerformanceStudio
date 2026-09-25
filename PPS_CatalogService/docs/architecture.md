# Catalog service N-tier architecture

The service uses four logical tiers deployed as one AWS Lambda artifact. MongoDB Atlas remains an external storage tier.

```text
Presentation -> Application/Business -> Ports <- Data Access -> MongoDB Atlas
                                      <- Image Store -> assets/*
```

## Presentation

`src/presentation` owns HTTP routing, request-body parsing, response serialization, and HTTP error mapping. It calls `ProductApplication`; it cannot access repositories or MongoDB.

## Application and business logic

`src/application` owns use cases and ports. `CatalogProductService` normalizes and validates inputs, verifies active Brand and Category references, verifies product image filenames, and coordinates persistence. It contains no MongoDB, Lambda, or filesystem access.

## Domain

`src/domain` owns entities, value objects, database-neutral string identifiers, component taxonomy, and business validation. It contains no Lambda, MongoDB, BSON, or filesystem types.

## Data access

`src/data` implements the application ports. MongoDB ObjectIds, BSON Decimal128, BSON timestamps, queries, indexes, validators, and duplicate-key mapping are confined to this tier. `FileSystemImageStore` is replaceable by a future S3 implementation.

## Composition and utilities

`src/bin/api.rs` is the composition root that wires concrete adapters to application services. `migrate` and `seed` are explicit administrative executables and never run during API cold starts.

## Enforced rules

- Presentation depends on application interfaces, not data implementations.
- Application depends on domain types and its own ports.
- Domain cannot import delivery or persistence frameworks.
- Data converts between domain and storage representations.
- Cross-tier behavior is tested at the owning tier.
