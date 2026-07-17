/**
 * MSW handler factories for the Bearer browser-token refresh endpoint
 * (design §4.4 — `POST /api/auth/browser-token/refresh`).
 *
 * These are **not** registered in the global default `handlers` export (see
 * `src/test/mocks/handlers.ts`) to avoid polluting other suites. The API client
 * interceptor tests import the factories here and install them ad-hoc via
 * `server.use(...)`, which lets each test control the exact response sequence
 * (success rotation, reuse-detected 401, absolute-expiry 401, etc.).
 *
 * The refresh endpoint rotates the refresh token family: a 200 returns a NEW
 * access token AND a NEW refresh token. A 401 means the presented refresh token
 * was invalid/expired/revoked, OR — critically — was reused (an old family
 * member presented again), which triggers backend family revocation. The
 * `error` string lets callers distinguish reuse vs ordinary expiry.
 */

import { http, HttpResponse } from 'msw'
import type { BrowserTokenResponse, RefreshBrowserTokenRequest } from '@/lib/api-generated'
import { makeRotatedBrowserTokenResponse, TOKEN_FIXTURE } from '@/test/fixtures/browser-token'

const API_BASE_URL = 'http://localhost:3000'
export const REFRESH_URL = `${API_BASE_URL}/api/auth/browser-token/refresh`

/**
 * Refresh-request body observed by the handler. Exposed so a test can assert the
 * interceptor sent the persisted refresh token + clientId (not a stale one).
 */
export interface CapturedRefreshRequest {
  body: RefreshBrowserTokenRequest
  authorization: string | null
}

/**
 * Build a handler that returns a rotated token set (200). Optionally captures
 * the request body + Authorization header for assertions.
 *
 * @param capture  - object to fill with the observed request (mutated).
 * @param response - override the rotated token set (e.g. custom new AT).
 */
export function createRefreshSuccessHandler(
  capture?: CapturedRefreshRequest,
  response?: BrowserTokenResponse
) {
  return http.post(REFRESH_URL, async ({ request }) => {
    if (capture) {
      capture.body = (await request.json()) as RefreshBrowserTokenRequest
      capture.authorization = request.headers.get('Authorization')
    }
    return HttpResponse.json(response ?? makeRotatedBrowserTokenResponse())
  })
}

/**
 * Standard refresh `error` strings the backend returns on a 401, per design
 * §4.4. The interceptor (`api-client.ts`) routes every refresh failure to a
 * clean re-login (it does not branch on the string itself), but tests use these
 * to assert the wire-level distinguishability the caller / backend relies on.
 */
export const REFRESH_ERRORS = {
  /** Old refresh token presented again → entire family revoked. */
  reuseDetected: 'refresh_reuse_detected',
  /** Refresh token past its absolute lifetime ceiling. */
  absoluteExpiry: 'refresh_absolute_expiry',
  /** Refresh token unknown / revoked / otherwise invalid. */
  invalid: 'invalid_refresh_token',
} as const

/**
 * Build a handler that fails refresh with a 401 carrying a distinguishable
 * `error` string. Used to assert the reuse/absolute-expiry → force-re-login
 * path (RT cleared, no silent failure, no infinite retry).
 */
export function createRefreshErrorHandler(
  error: (typeof REFRESH_ERRORS)[keyof typeof REFRESH_ERRORS] | string,
  status: 401 | 400 = 401
) {
  return http.post(REFRESH_URL, () =>
    HttpResponse.json({ error, error_description: error }, { status })
  )
}

/**
 * Build a handler whose response mutates on each call — used to script a
 * sequence (e.g. 200 then 401, or 401 then 200) for loop-guard tests.
 */
export function createSequencedRefreshHandler(
  responses: Array<{ status: number; body?: unknown }>
) {
  let i = 0
  return http.post(REFRESH_URL, () => {
    const next = responses[Math.min(i, responses.length - 1)]
    i += 1
    return HttpResponse.json(next.body ?? {}, { status: next.status })
  })
}

/**
 * The canonical rotated token pair, re-exported for tests that assert the
 * interceptor wrote the rotated AT/RT to the holder/store after a refresh.
 */
export { TOKEN_FIXTURE }
