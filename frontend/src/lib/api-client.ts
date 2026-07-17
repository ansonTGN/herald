/**
 * Bearer API client wiring (design §4.4 — Herald own frontend token model).
 *
 * This is the single hand-maintained module that adapts the generated
 * `@hey-api` client to the Bearer access/refresh token family:
 *
 *   - a **request interceptor** injects `Authorization: Bearer {accessToken}`
 *     from the in-memory holder onto every generated-client request;
 *   - a **response interceptor** catches a single 401, calls the refresh
 *     endpoint once, swaps the tokens, and replays the original request
 *     exactly once (loop-guarded).
 *
 * It is deliberately thin: it imports only the generated `client`, the token
 * holder from the auth store, and the refresh wrapper from auth-service. It
 * hardcodes no API paths — all network calls go through generated SDK
 * functions (`refresh` via `refreshBrowserToken`).
 *
 * `initBearerClient()` is called from `main.tsx` before `createRouter`/render.
 */

import { client } from '@/lib/api-generated/client.gen'
import type { ResolvedRequestOptions } from '@/lib/api-generated/client/types.gen'
import { accessTokenHolder, useAuthStore } from '@/stores/auth-store'
import { refreshBrowserToken } from '@/lib/auth-service'

/** Interceptor argument types (the generated client is a fetch client, so the
 *  request/response generic params are the DOM `Request`/`Response`). */
type ReqOptions = ResolvedRequestOptions

/** Marker header so the response interceptor only retries each request once. */
const RETRY_HEADER = 'X-Herald-Refresh-Retried'
/** The path of the refresh endpoint, to avoid refreshing the refresh call. */
const REFRESH_PATH = '/api/auth/browser-token/refresh'

let initialized = false
/** Single-flight: concurrent 401s share one in-flight refresh promise. */
let refreshInFlight: Promise<boolean> | null = null

/**
 * Run the refresh exactly once for a batch of concurrent 401s. Resolves to
 * `true` if a new access token was obtained (callers may retry), `false` if
 * refresh failed (callers must surface re-login).
 */
function refreshOnce(): Promise<boolean> {
  if (refreshInFlight) return refreshInFlight
  const { refreshToken, clientId } = useAuthStore.getState().getRefreshToken()
  if (!refreshToken || !clientId) {
    return Promise.resolve(false)
  }
  refreshInFlight = (async () => {
    try {
      const tokenSet = await refreshBrowserToken(refreshToken, clientId)
      useAuthStore.getState().setTokens({
        accessToken: tokenSet.accessToken,
        refreshToken: tokenSet.refreshToken,
        clientId,
      })
      return true
    } catch {
      // Refresh reuse / expired / revoked → force a clean re-login (design §4.2.2,
      // §4.5). Clear all token material so the next status check 401s.
      useAuthStore.getState().logout()
      return false
    } finally {
      refreshInFlight = null
    }
  })()
  return refreshInFlight
}

/**
 * Replay the original request after a successful refresh. The generated client
 * builds a fresh `Request` for each call via the SDK function that produced it,
 * so we cannot naively re-`fetch` the old `Request` (its body stream is
 * already consumed). Instead we re-issue through `client.request` with the same
 * options, which rebuilds the request with the now-updated Authorization.
 */
async function replayRequest(options: Record<string, unknown>): Promise<Response> {
  // `client.request` returns the `{ data, error, request, response }` envelope
  // (responseStyle 'fields', the default). It also fully consumes + parses the
  // response body internally (success → `data`, error → `error`). We cannot
  // return `result.response` directly: the OUTER request pipeline that invoked
  // this interceptor will try to read the body AGAIN, and a `Response` body can
  // only be read once ("Body is unusable: Body has already been read"). So
  // reconstruct a fresh Response from the already-parsed payload.
  const result = (await client.request(options as never)) as {
    response?: Response
    data?: unknown
    error?: unknown
  }
  if (!result.response) {
    // No response object (e.g. network error path) — synthesize a 401 so the
    // caller surfaces re-login rather than masking the failure.
    return new Response(null, { status: 401 })
  }
  const status = result.response.status
  const headers = new Headers(result.response.headers)
  const payload = result.data !== undefined ? result.data : result.error
  const body = payload !== undefined && payload !== null ? JSON.stringify(payload) : null
  return new Response(body, { status, headers })
}

/**
 * Install the Bearer request interceptor and the 401 silent-refresh retry
 * interceptor on the generated client. Idempotent — safe to call once from
 * `main.tsx`.
 */
export function initBearerClient(): void {
  if (initialized) return
  initialized = true

  // --- Request interceptor: inject Authorization: Bearer ---
  client.interceptors.request.use((request: Request, options: ReqOptions) => {
    // Skip the refresh endpoint itself — it authenticates with the refresh
    // token in the body, not a Bearer access token.
    const url = options.url
    if (url === REFRESH_PATH) {
      return request
    }
    const accessToken = accessTokenHolder.get()
    if (!accessToken) {
      return request
    }
    // Reuse the same Request object but add the header. `Request` is immutable
    // for headers, so we clone with the authorization set.
    const headers = new Headers(request.headers)
    // Only set if not already present (e.g. a request that pre-set its own).
    if (!headers.has('Authorization')) {
      headers.set('Authorization', `Bearer ${accessToken}`)
    }
    return new Request(request, { headers })
  })

  // --- Response interceptor: single 401 → refresh → retry once ---
  client.interceptors.response.use(
    async (response: Response, request: Request, options: ReqOptions) => {
      if (response.status !== 401) {
        return response
      }
      const url = options.url
      // Never auto-refresh the refresh endpoint, and never retry a request twice.
      if (url === REFRESH_PATH) {
        return response
      }
      const retried = request.headers.get(RETRY_HEADER)
      if (retried) {
        return response
      }

      const refreshed = await refreshOnce()
      if (!refreshed) {
        // Refresh failed; let the 401 propagate so the caller routes to re-login.
        return response
      }

      // Mark the replay so a subsequent 401 is not refreshed again (loop guard).
      // Preserve the original per-request headers (a `Headers` instance by this
      // stage — spreading it as a plain object would drop them) and add the marker.
      const replayOptions: Record<string, unknown> = { ...options }
      const headers = new Headers(
        (replayOptions.headers as Headers | Record<string, string> | undefined) ?? undefined
      )
      headers.set(RETRY_HEADER, '1')
      replayOptions.headers = headers
      return replayRequest(replayOptions)
    }
  )
}
