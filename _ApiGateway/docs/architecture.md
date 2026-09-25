# Architecture decisions

## Runtime request path

`Client -> KrakenD :8080 -> Docker DNS / service discovery -> PPS microservice`

KrakenD is stateless. Multiple gateway replicas can sit behind a cloud or local load balancer. Docker's embedded DNS resolves service names locally; production can replace each static backend with the included Consul-compatible DNS SRV pattern. KrakenD then refreshes the discovered instances and load-balances them using SRV priority and weight.

## Authentication decision

The Auth microservice does not have to be implemented before the gateway. During local development, `PPS_AUTH_MODE=development` forwards the Catalog service's existing `X-PPS-Admin-Key`. This preserves the current service boundary.

When Auth publishes an RS256 JWKS document, set `PPS_AUTH_MODE=jwt`. KrakenD then validates signature, issuer, expiration, and the Cognito custom scope `pps-api/catalog.admin` before forwarding an admin request. The gateway passes the verified subject as `X-PPS-User-ID`. Services should still authorize sensitive domain actions; edge validation is not a replacement for service-level authorization.

## Capability mapping

| Requirement | Implementation |
|---|---|
| Reverse proxy/router | Versioned `/api/v1/catalog/*` endpoints map to the Catalog service contract |
| Service discovery | Docker DNS locally; DNS SRV/Consul example for dynamic environments |
| Load balancing | KrakenD round robin for static host arrays; SRV priority/weight for Consul |
| Authentication | Conditional RS256 JWT/JWKS validation; development admin header compatibility |
| Rate limiting | Token buckets per endpoint and per client IP |
| Response cache | Bounded in-memory RFC-aware cache on safe Catalog reads |
| Protocol translation | REST-to-gRPC configuration pattern included for future gRPC services |
| Observability | Structured container logs, Prometheus metrics, OTLP traces |

## Production hardening backlog

- Put TLS termination at the ingress/load balancer and restrict ports 9091 and 4317 to the observability network.
- Replace localhost CORS origins with the deployed PPS web origins.
- Export OTLP to the production tracing backend instead of the debug collector.
- Use a distributed rate-limit service if globally exact quotas are required; Community Edition counters are per gateway instance.
- Pin images by digest in release environments and add container/image scanning in CI.
