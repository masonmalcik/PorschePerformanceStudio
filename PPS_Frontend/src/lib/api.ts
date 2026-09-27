const gatewayOrigin = (import.meta.env.PUBLIC_PPS_API_GATEWAY_URL ?? 'http://localhost:8080').replace(/\/$/, '');

export const catalogApiBase = `${gatewayOrigin}/api/v1/catalog`;
export const catalogAdminApiBase = `${catalogApiBase}/admin`;
export const developmentAdminKey = import.meta.env.PUBLIC_PPS_ADMIN_KEY?.trim() ?? '';
export const cartApiBase = `${gatewayOrigin}/api/v1/cart`;
export const orderApiBase = `${gatewayOrigin}/api/v1/orders`;

export function adminHeaders(json = false): Record<string, string> {
  const headers: Record<string, string> = {};
  const accessToken = import.meta.env.PUBLIC_PPS_ACCESS_TOKEN?.trim();

  if (json) headers['Content-Type'] = 'application/json';
  if (accessToken) headers.Authorization = `Bearer ${accessToken}`;
  else if (developmentAdminKey) headers['x-pps-admin-key'] = developmentAdminKey;

  return headers;
}
