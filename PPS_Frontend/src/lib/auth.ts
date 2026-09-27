const SESSION_KEY = 'pps.auth.session';
const TRANSACTION_KEY = 'pps.auth.transaction';
const AUTH_EVENT = 'pps:auth-changed';

export const authApiBase = (import.meta.env.PUBLIC_PPS_AUTH_API_URL ?? 'https://tx40tjevg0.execute-api.us-east-1.amazonaws.com/dev').replace(/\/$/, '');
export const cognitoDomain = (import.meta.env.PUBLIC_COGNITO_DOMAIN ?? 'https://pps-024080483599-dev.auth.us-east-1.amazoncognito.com').replace(/\/$/, '');
export const cognitoClientId = import.meta.env.PUBLIC_COGNITO_CLIENT_ID ?? '1n58iicfsjglca4bghrvjhon8q';
const scopes = ['openid', 'email', 'profile', 'pps-api/profile.read', 'pps-api/profile.write', 'pps-api/catalog.admin'];

export type AuthSession = {
  accessToken: string;
  idToken: string;
  refreshToken: string;
  expiresAt: number;
  scope: string;
  subject: string;
  email: string;
  username: string;
  groups: string[];
};

type TokenPayload = { sub?: string; email?: string; username?: string; 'cognito:username'?: string; 'cognito:groups'?: string[]; scope?: string; exp?: number };
type TokenResponse = { access_token: string; id_token?: string; refresh_token?: string; expires_in: number; scope?: string; token_type: string };

export function getSession(): AuthSession | null {
  if (typeof sessionStorage === 'undefined') return null;
  try { return JSON.parse(sessionStorage.getItem(SESSION_KEY) ?? 'null') as AuthSession | null; } catch { return null; }
}

export function hasScope(scope: string): boolean {
  return (getSession()?.scope.split(/\s+/) ?? []).includes(scope);
}

export function isAdmin(): boolean {
  const session = getSession();
  return Boolean(session && session.groups.includes('pps-admins') && hasScope('pps-api/catalog.admin'));
}

export async function beginAuthentication(mode: 'login' | 'signup' | 'recovery' = 'login', returnTo = appPath('/account')): Promise<void> {
  const verifier = randomString(64);
  const state = randomString(32);
  const challenge = await sha256(verifier);
  const redirectUri = `${window.location.origin}${appPath('/auth/callback')}`;
  sessionStorage.setItem(TRANSACTION_KEY, JSON.stringify({ verifier, state, returnTo, redirectUri }));
  const query = new URLSearchParams({ response_type: 'code', client_id: cognitoClientId, redirect_uri: redirectUri, scope: scopes.join(' '), state, code_challenge: challenge, code_challenge_method: 'S256' });
  if (mode === 'login') {
    window.location.assign(`${authApiBase}/oauth/authorize?${new URLSearchParams({ redirect_uri: redirectUri, state, code_challenge: challenge })}`);
  } else {
    window.location.assign(`${cognitoDomain}/${mode === 'signup' ? 'signup' : 'forgotPassword'}?${query}`);
  }
}

export async function completeAuthentication(code: string, state: string): Promise<string> {
  const transaction = JSON.parse(sessionStorage.getItem(TRANSACTION_KEY) ?? 'null') as { verifier: string; state: string; returnTo: string; redirectUri: string } | null;
  if (!transaction || transaction.state !== state) throw new Error('The authentication request could not be verified. Please start again.');
  const response = await fetch(`${authApiBase}/oauth/token`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ code, codeVerifier: transaction.verifier, redirectUri: transaction.redirectUri }) });
  if (!response.ok) throw new Error('Cognito could not complete sign-in. Please try again.');
  saveTokens(await response.json() as TokenResponse);
  sessionStorage.removeItem(TRANSACTION_KEY);
  return safeReturnTo(transaction.returnTo);
}

let refreshInFlight: Promise<AuthSession | null> | null = null;
export async function refreshSession(): Promise<AuthSession | null> {
  if (refreshInFlight) return refreshInFlight;
  const current = getSession();
  if (!current?.refreshToken) return null;
  refreshInFlight = (async () => {
    const response = await fetch(`${authApiBase}/oauth/token`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ grantType: 'refresh_token', refreshToken: current.refreshToken }) });
    if (!response.ok) { clearSession(); return null; }
    return saveTokens(await response.json() as TokenResponse, current);
  })().finally(() => { refreshInFlight = null; });
  return refreshInFlight;
}

export async function getValidAccessToken(): Promise<string | null> {
  const session = getSession();
  if (!session) return null;
  if (session.expiresAt > Date.now() + 60_000) return session.accessToken;
  return (await refreshSession())?.accessToken ?? null;
}

export async function authenticatedFetch(input: RequestInfo | URL, init: RequestInit = {}): Promise<Response> {
  const headers = new Headers(init.headers);
  const token = await getValidAccessToken();
  if (token) headers.set('Authorization', `Bearer ${token}`);
  let response = await fetch(input, { ...init, headers });
  if (response.status === 401 && getSession()?.refreshToken) {
    const refreshed = await refreshSession();
    if (refreshed) { headers.set('Authorization', `Bearer ${refreshed.accessToken}`); response = await fetch(input, { ...init, headers }); }
  }
  return response;
}

export function installAuthenticatedFetch(): void {
  if (typeof window === 'undefined' || (window as unknown as { __ppsFetch?: boolean }).__ppsFetch) return;
  const nativeFetch = window.fetch.bind(window);
  (window as unknown as { __ppsFetch?: boolean }).__ppsFetch = true;
  window.fetch = async (input, init = {}) => {
    const url = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url;
    if (url.includes('/oauth/')) return nativeFetch(input, init);
    const headers = new Headers(init.headers ?? (input instanceof Request ? input.headers : undefined));
    const token = await getValidAccessToken();
    if (token) headers.set('Authorization', `Bearer ${token}`);
    let response = await nativeFetch(input, { ...init, headers });
    if (response.status === 401 && getSession()?.refreshToken) {
      const refreshed = await refreshSession();
      if (refreshed) { headers.set('Authorization', `Bearer ${refreshed.accessToken}`); response = await nativeFetch(input, { ...init, headers }); }
    }
    return response;
  };
}

export function signOut(): void {
  clearSession();
  const logoutUri = `${window.location.origin}${appPath('/')}`;
  window.location.assign(`${authApiBase}/oauth/logout?${new URLSearchParams({ logout_uri: logoutUri })}`);
}

export function clearSession(): void { if (typeof sessionStorage !== 'undefined') sessionStorage.removeItem(SESSION_KEY); notify(); }
export function onAuthChange(handler: () => void): () => void { window.addEventListener(AUTH_EVENT, handler); window.addEventListener('storage', handler); return () => { window.removeEventListener(AUTH_EVENT, handler); window.removeEventListener('storage', handler); }; }

function saveTokens(tokens: TokenResponse, previous?: AuthSession): AuthSession {
  const access = decode(tokens.access_token);
  const identity = tokens.id_token ? decode(tokens.id_token) : {};
  const session: AuthSession = {
    accessToken: tokens.access_token,
    idToken: tokens.id_token ?? previous?.idToken ?? '',
    refreshToken: tokens.refresh_token ?? previous?.refreshToken ?? '',
    expiresAt: Date.now() + tokens.expires_in * 1000,
    scope: tokens.scope ?? access.scope ?? previous?.scope ?? '',
    subject: access.sub ?? identity.sub ?? '',
    email: identity.email ?? access.email ?? previous?.email ?? '',
    username: access.username ?? access['cognito:username'] ?? identity['cognito:username'] ?? previous?.username ?? '',
    groups: access['cognito:groups'] ?? previous?.groups ?? [],
  };
  sessionStorage.setItem(SESSION_KEY, JSON.stringify(session)); notify(); return session;
}
function decode(token: string): TokenPayload {
  try {
    const payload = token.split('.')[1];
    if (!payload) return {};
    const base64 = payload.replace(/-/g, '+').replace(/_/g, '/').padEnd(Math.ceil(payload.length / 4) * 4, '=');
    return JSON.parse(atob(base64)) as TokenPayload;
  } catch { return {}; }
}
function notify(): void { if (typeof window !== 'undefined') window.dispatchEvent(new Event(AUTH_EVENT)); }
function randomString(length: number): string { const bytes = crypto.getRandomValues(new Uint8Array(length)); return base64url(bytes); }
async function sha256(value: string): Promise<string> { return base64url(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value)))); }
function base64url(bytes: Uint8Array): string { return btoa(String.fromCharCode(...bytes)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, ''); }
function safeReturnTo(value: string): string { return value.startsWith('/') && !value.startsWith('//') ? value : appPath('/account'); }
import { appPath } from './navigation';
