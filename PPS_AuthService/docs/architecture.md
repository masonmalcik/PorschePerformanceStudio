# PPS Auth architecture

## Trust boundaries

Cognito User Pools owns credentials, password policy, MFA, recovery, email verification, sessions, OAuth grants, token signing, and JWKS rotation. The Go service never receives or stores a password. DynamoDB stores only application profile fields keyed by the immutable Cognito `sub` claim.

The browser uses OAuth 2.0 Authorization Code with PKCE. `/oauth/authorize` validates the callback allowlist, state, and S256 challenge before redirecting to Cognito managed login. `/oauth/token` exchanges the one-time code and verifier. `/oauth/logout` validates the sign-out destination.

`/me` calls Cognito `userInfo` with the access token and uses the returned `sub` as the DynamoDB partition key. This provides defense in depth when the service is invoked without KrakenD. KrakenD independently validates the access-token signature, issuer, expiration, and scopes. Cognito places the app client identifier in the access token's `client_id` claim rather than the ID-token `aud` claim.

## DynamoDB model

Table partition key: `userId` (Cognito `sub`). One item per user:

- `email`
- `displayName`
- `locale`
- `timezone`
- `marketingOptIn`
- `createdAt`
- `updatedAt`

The table uses on-demand billing, AWS-owned encryption at rest, and retain-on-stack-delete protection. No scans or secondary indexes are required.

## OAuth scopes

- `openid`, `email`, `profile`: OIDC identity information
- `pps-api/profile.read`: read the PPS profile
- `pps-api/profile.write`: update the PPS profile
- `pps-api/catalog.admin`: administer Catalog resources through KrakenD

## Request flow

```text
Browser -> Go /oauth/authorize -> Cognito managed login
Browser <- authorization code <- Cognito
Browser -> Go /oauth/token (code + PKCE verifier) -> Cognito token endpoint
Browser -> KrakenD (Bearer access token) -> PPS services
Browser -> Go /me -> Cognito userInfo + DynamoDB profile
```
