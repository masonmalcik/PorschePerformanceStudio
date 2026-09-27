/// <reference types="astro/client" />

interface ImportMetaEnv {
  readonly PUBLIC_PPS_API_GATEWAY_URL?: string;
  readonly PUBLIC_PPS_ADMIN_KEY?: string;
  readonly PUBLIC_PPS_ACCESS_TOKEN?: string;
  readonly PUBLIC_PPS_AUTH_API_URL?: string;
  readonly PUBLIC_COGNITO_DOMAIN?: string;
  readonly PUBLIC_COGNITO_CLIENT_ID?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
