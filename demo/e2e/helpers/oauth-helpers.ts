/**
 * OAuth PKCE API helpers for E2E tests.
 *
 * The Location header from oauthAuthorize is a **relative URL**.
 * Callers must prepend `baseUrl` before using it with `page.goto()`.
 *
 * seedOAuthClientApp uses `request` (APIRequestContext) which inherits
 * the browser context's auth cookies. Call AFTER the page is authenticated.
 */

import { type APIRequestContext, type Page, type Response, expect } from '@playwright/test'
import * as crypto from 'node:crypto'
import { BASE_URL } from './environment-setup'

export { BASE_URL }

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface OAuthAuthorizeResult {
  status: number
  redirectLocation: string | null
  errorBody?: string
}

export interface OAuthTokenResponse {
  access_token: string
  token_type: string
  expires_in: number
}

export interface OAuthTokenErrorResponse {
  error: string
  error_description: string
}

export interface OAuthPKCEPair {
  code_verifier: string
  code_challenge: string
}

export interface SeedOAuthClientAppResult {
  clientId: string
  appId: string
}

// ---------------------------------------------------------------------------
// PKCE Cryptographic Utilities
// ---------------------------------------------------------------------------

export function generatePKCEPair(): OAuthPKCEPair {
  const code_verifier = crypto.randomBytes(32).toString('base64url')
  const code_challenge = crypto
    .createHash('sha256')
    .update(code_verifier)
    .digest('base64url')

  return { code_verifier, code_challenge }
}

// ---------------------------------------------------------------------------
// Authorize URL Builder
// ---------------------------------------------------------------------------

export function buildAuthorizeUrl(
  baseUrl: string,
  realmId: string,
  params: {
    client_id: string
    redirect_uri: string
    state: string
    code_challenge: string
  },
): string {
  const query = new URLSearchParams({
    client_id: params.client_id,
    redirect_uri: params.redirect_uri,
    state: params.state,
    response_type: 'code',
    code_challenge: params.code_challenge,
    code_challenge_method: 'S256',
  })

  return `${baseUrl}/api/oauth/${encodeURIComponent(realmId)}/authorize?${query.toString()}`
}

// ---------------------------------------------------------------------------
// OAuth Authorize (capture 302 redirect)
// ---------------------------------------------------------------------------

export async function oauthAuthorize(
  baseUrl: string = BASE_URL,
  realmId: string,
  params: {
    client_id: string
    redirect_uri: string
    state: string
    code_challenge: string
  },
): Promise<OAuthAuthorizeResult> {
  const url = buildAuthorizeUrl(baseUrl, realmId, params)

  const response = await fetch(url, {
    method: 'GET',
    redirect: 'manual',
  })

  const status = response.status

  if (status === 302) {
    return { status, redirectLocation: response.headers.get('location') }
  }

  const errorBody = await response.text()
  return { status, redirectLocation: null, errorBody }
}

// ---------------------------------------------------------------------------
// OAuth Token Exchange
// ---------------------------------------------------------------------------

export async function oauthTokenExchange(
  baseUrl: string = BASE_URL,
  realmId: string,
  params: {
    grant_type: string
    code: string
    redirect_uri: string
    client_id: string
    code_verifier: string
  },
): Promise<OAuthTokenResponse | OAuthTokenErrorResponse> {
  const response = await fetch(`${baseUrl}/api/oauth/${encodeURIComponent(realmId)}/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(params),
  })

  const data = await response.json()
  return response.ok
    ? data as OAuthTokenResponse
    : data as OAuthTokenErrorResponse
}

// ---------------------------------------------------------------------------
// Client App Seed
// ---------------------------------------------------------------------------

export async function seedOAuthClientApp(
  request: APIRequestContext,
  realmId: string,
  options?: {
    appName?: string
    redirectUris?: string[]
  },
  baseUrl: string = BASE_URL,
): Promise<SeedOAuthClientAppResult> {
  const appName = options?.appName ?? 'OAuth PKCE Test App'
  const redirectUris = options?.redirectUris ?? ['https://example.com/oauth/callback']
  const clientId = `oauth-test-${Date.now()}`

  const response = await request.post(`${baseUrl}/api/client/${encodeURIComponent(realmId)}`, {
    data: {
      clientId,
      name: appName,
      description: 'Auto-created client app for OAuth PKCE demo tests',
      redirectUris,
      enabled: true,
      sessionTtlSeconds: 1800,
      deviceCodeGrantEnabled: false,
    },
  })

  if (!response.ok()) {
    const body = await response.text()
    throw new Error(
      `[oauth-helpers] Failed to seed OAuth client app: ${response.status()} ${body}`,
    )
  }

  const data = await response.json()
  return { clientId, appId: data.id as string }
}

// ---------------------------------------------------------------------------
// Shared UI helpers for OAuth login flow
// ---------------------------------------------------------------------------

/** Block navigation to unreachable OAuth callback URLs in test environment. */
export async function blockExternalCallback(page: Page): Promise<void> {
  await page.route('https://example.com/**', (route) => route.abort())
}

/** Predicate matching the login API POST response. */
export function isLoginApiResponse(resp: Response): boolean {
  return resp.url().includes('/api/auth/') && resp.url().includes('/login') && resp.request().method() === 'POST'
}

/**
 * Complete the full authorize + login + extract auth code flow.
 * Returns the auth code and PKCE pair for subsequent token exchange.
 */
export async function completeOAuthLoginAndGetAuthCode(
  page: Page,
  baseUrl: string,
  realmId: string,
  clientId: string,
  redirectUri: string,
  state: string,
  credentials: { email: string; password: string },
): Promise<{
  authCode: string
  pkce: OAuthPKCEPair
}> {
  const pkce = generatePKCEPair()

  const authorizeResult = await oauthAuthorize(baseUrl, realmId, {
    client_id: clientId,
    redirect_uri: redirectUri,
    state,
    code_challenge: pkce.code_challenge,
  })

  expect(authorizeResult.status).toBe(302)
  expect(authorizeResult.redirectLocation).toBeTruthy()

  await page.goto(`${baseUrl}${authorizeResult.redirectLocation}`, { waitUntil: 'domcontentloaded' })
  await expect(page.getByTestId('login-card')).toBeVisible({ timeout: 10000 })

  await blockExternalCallback(page)

  const loginResponsePromise = page.waitForResponse(isLoginApiResponse, { timeout: 15000 })

  await page.getByTestId('email-input').fill(credentials.email)
  await page.getByTestId('password-input').fill(credentials.password)
  await page.getByTestId('login-submit-button').click()

  const loginResponse = await loginResponsePromise
  expect(loginResponse.ok()).toBe(true)

  const loginData = await loginResponse.json()
  const redirectTo: string = loginData.redirectTo
  expect(redirectTo).toContain('code=')

  const authCode = new URL(redirectTo).searchParams.get('code')!
  expect(authCode).toBeTruthy()

  return { authCode, pkce }
}
