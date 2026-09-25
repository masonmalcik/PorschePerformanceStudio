# PPS Auth Service

Go authentication/profile microservice for Porsche Performance Studio. Amazon Cognito User Pools supplies OIDC/OAuth 2.0, managed login, MFA, recovery, and signed access tokens. DynamoDB stores only PPS application-profile data.

## API

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/health` | Liveness check |
| `GET` | `/.well-known/openid-configuration` | Proxies Cognito discovery metadata |
| `GET` | `/.well-known/jwks.json` | Proxies Cognito signing keys |
| `GET` | `/oauth/authorize` | Starts Authorization Code + PKCE |
| `POST` | `/oauth/token` | Exchanges code and PKCE verifier |
| `GET` | `/oauth/logout` | Starts Cognito logout |
| `GET` | `/me` | Returns Cognito identity plus PPS profile |
| `PUT` | `/me` | Updates PPS profile fields |

The service listens on port `8082` locally.

## AWS deployment

Prerequisites: Go 1.24+, AWS SAM CLI, and authenticated AWS credentials.

```powershell
sam build
sam deploy --guided
```

During guided deployment, choose a globally unique `CognitoDomainPrefix`. The SAM stack creates the User Pool, public PKCE app client, resource server/scopes, managed-login domain, DynamoDB table, HTTP API, and ARM64 Lambda.

Use the stack outputs to configure the gateway:

```dotenv
PPS_AUTH_MODE=jwt
PPS_JWKS_URL=<JWKSURL output>
PPS_JWT_ISSUER=<Issuer output>
```

KrakenD's Catalog admin routes must require the Cognito custom scope `pps-api/catalog.admin`. Cognito access tokens identify the app client with the `client_id` claim rather than the ID-token `aud` claim, so the gateway is configured to authorize API access with issuer, signature, expiry, and scope validation.

## Local container

Local execution still uses the real AWS User Pool and DynamoDB table. Copy `.env.example` to `.env`, insert deployed stack outputs, and provide AWS credentials through Docker Desktop/your environment.

```powershell
docker network inspect pps *> $null
if ($LASTEXITCODE -ne 0) { docker network create pps }
Copy-Item .env.example .env
docker compose up --build -d
Invoke-RestMethod http://localhost:8082/health
```

## PKCE browser flow

Generate a fresh random verifier of 43-128 unreserved characters and its base64url SHA-256 challenge. Redirect the browser to:

```text
GET /oauth/authorize?redirect_uri=<approved URL>&state=<random CSRF value>&code_challenge=<challenge>
```

After Cognito redirects back with `code` and `state`, verify the state and exchange the code:

```json
POST /oauth/token
{
  "code": "authorization-code",
  "codeVerifier": "original-random-verifier",
  "redirectUri": "http://localhost:4321/auth/callback"
}
```

Do not persist access or refresh tokens in `localStorage`. Prefer secure, HTTP-only, same-site cookies through a backend-for-frontend before production launch.

## Development checks

```powershell
go mod tidy
go test ./...
go vet ./...
```

See `docs/architecture.md` for trust boundaries, data ownership, scopes, and request flows.
