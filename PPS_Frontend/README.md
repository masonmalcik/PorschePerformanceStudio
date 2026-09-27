# PPS Frontend

Astro and React frontend for the Porsche Performance Studio microservices stack.

## Local development

```powershell
Copy-Item .env.example .env
npm ci
npm run dev
```

Open `http://localhost:4321`. The checked-in defaults use the deployed Cognito/Auth service and the local KrakenD gateway. Set `PUBLIC_PPS_API_BASE_URL` to the deployed unified HTTP API when testing the AWS backend directly.

Authentication uses Cognito Authorization Code flow with PKCE. Tokens are kept in `sessionStorage`, attached automatically to API requests, and refreshed through the Auth service. `/account`, `/cart`, `/checkout`, `/order`, and `/admin/*` are protected in the browser. Admin pages additionally require membership in the Cognito `pps-admins` group and the `pps-api/catalog.admin` scope; the Catalog service enforces the same authorization server-side.

## Production build

```powershell
npm ci
npm run build
```

The static output is written to `dist/`.
