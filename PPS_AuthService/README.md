# PPS Auth Service

Go authentication and authorization microservice for Porsche Performance Studio. Amazon Cognito User Pools supplies OIDC/OAuth 2.0, managed login, MFA, recovery, and signed access tokens. DynamoDB stores PPS profiles, roles, permissions, memberships, identity links, consent records, audit events, and migration history.

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
| `GET` | `/roles` | Lists roles; requires `roles:read` |
| `POST` | `/roles` | Creates a custom role; requires `roles:manage` |
| `GET` | `/roles/{roleId}` | Returns a role and its permissions |
| `PUT` | `/roles/{roleId}/permissions` | Replaces role permissions; requires `roles:manage` |
| `PUT` | `/users/{userId}/roles/{roleId}` | Assigns a role; requires `roles:assign` |
| `DELETE` | `/users/{userId}/roles/{roleId}` | Removes a role; requires `roles:assign` |
| `GET` | `/organizations` | Lists organizations |
| `POST` | `/organizations` | Creates an organization |
| `PUT` | `/organizations/{organizationId}/members/{userId}` | Creates or updates a membership |
| `GET` | `/me/consents` | Returns the signed-in user's consent history |
| `POST` | `/me/consents` | Appends an immutable consent record |

The service listens on port `8082` locally.

## Token verification

Protected routes verify Cognito access tokens locally with RS256 and require the configured issuer, app-client ID, `access` token use, expiration, subject, and signing key ID. Verified OIDC/Cognito claims are attached to the Go request context; protected handlers do not call Cognito's `userInfo` endpoint.

The in-memory JWKS cache refreshes every 24 hours and refreshes early when Cognito rotates to an unknown signing key. JWKS requests use a dedicated three-second HTTP timeout and three bounded attempts with backoff. When Cognito's keys cannot be refreshed, protected routes return retryable HTTP `503` responses instead of panicking or misclassifying the outage as an invalid credential.

## AWS deployment

Prerequisites: Go 1.24+, AWS SAM CLI, and authenticated AWS credentials.

```powershell
sam build
sam deploy --guided
```

During guided deployment, choose a globally unique `CognitoDomainPrefix`. The SAM stack creates the User Pool, public PKCE app client, resource server/scopes, managed-login domain, DynamoDB table, HTTP API, and x86-64 Lambda.

## Database migrations

Infrastructure schema changes are managed by SAM/CloudFormation. Versioned data migrations are implemented in `cmd/migrate` and use a DynamoDB lock plus checksummed migration records. Run migrations after deploying the table:

```powershell
docker run --rm `
  -v "${PWD}:/src" `
  -v "${env:USERPROFILE}\.aws:/root/.aws:ro" `
  -w /src `
  -e AWS_PROFILE=pps-deploy `
  -e AWS_REGION=us-east-1 `
  golang:1.24 `
  go run ./cmd/migrate --table pps-auth-dev --region us-east-1
```

The initial migrations seed permissions, system roles, and role-permission mappings. New users receive the `customer` role atomically when their profile is created.

### Initial administrator

There is intentionally no public first-administrator endpoint. After the first user signs in once (creating their profile), find their Cognito `sub` and use the credentialed bootstrap command:

```powershell
docker run --rm `
  -v "${PWD}:/src" `
  -v "${env:USERPROFILE}\.aws:/root/.aws:ro" `
  -w /src `
  -e AWS_PROFILE=pps-deploy `
  -e AWS_REGION=us-east-1 `
  golang:1.24 `
  go run ./cmd/authctl --table pps-auth-dev --user-id <COGNITO_SUB> --role administrator
```

Role, permission, organization, membership, and consent mutations write an immutable audit event in the same DynamoDB transaction as the changed record.

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
