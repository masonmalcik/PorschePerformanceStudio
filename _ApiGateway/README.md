# PPS API Gateway

Docker-first KrakenD Community Edition gateway for the Porsche Performance Studio microservices stack.

## Prerequisites

- Docker Desktop using Linux containers and the WSL 2 backend
- Docker Compose v2

## Start locally

Create the shared network once, copy the environment template, and start the gateway:

```powershell
docker network inspect pps *> $null; if ($LASTEXITCODE -ne 0) { docker network create pps }
Copy-Item .env.example .env
docker compose up -d
```

The Catalog service must be attached to the external `pps` network with the DNS name `catalog-service` and listen on port `8081`. If it runs on the Windows host instead, set `PPS_CATALOG_URL=http://host.docker.internal:<port>` in `.env`.

- Gateway health: `http://localhost:8080/__health`
- Catalog health: `http://localhost:8080/api/v1/catalog/health`
- Metrics: `http://localhost:9091/metrics`

Start the optional local observability services with:

```powershell
docker compose --profile observability up -d
```

Prometheus is then available at `http://localhost:9090`. The local collector prints OTLP traces to its logs.

## Authentication modes

The default `development` mode forwards `X-PPS-Admin-Key` to match the current Catalog service. Do not use that mode in a deployed environment.

After an Auth service exists, configure its JWKS endpoint and switch modes:

```dotenv
PPS_AUTH_MODE=jwt
PPS_JWKS_URL=http://auth-service:8082/.well-known/jwks.json
PPS_JWT_ISSUER=https://auth.pps.local
```

Admin JWTs must include the Cognito custom scope `pps-api/catalog.admin`. Public Catalog reads remain anonymous.

## Validate and test

```powershell
./scripts/validate.ps1
./scripts/smoke-test.ps1
```

`validate.ps1` renders the flexible configuration, checks it against KrakenD's embedded schema, and tests for route collisions. The smoke test expects both the gateway and Catalog service to be running.

See `docs/architecture.md` for the capability map, security decisions, service discovery design, and production backlog.
