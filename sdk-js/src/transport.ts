/**
 * Transport layer (design §5.4 / DEC-js-sdk-007): wraps a generated fetch client
 * instance with the Bearer access/refresh token model.
 *
 *   - request interceptor: inject `Authorization: Bearer {accessToken}` from
 *     the in-memory holder (skips the refresh endpoint);
 *   - response interceptor: on a single 401, run a single-flight refresh, swap
 *     the rotated tokens, and replay the original request exactly once
 *     (loop-guarded with `X-Herald-Refresh-Retried`).
 *
 * Each `createHeraldClient` gets its own generated client instance, so multiple
 * clients (and tests) are fully isolated. This is a direct port of the Herald
 * own-frontend wiring (`frontend/src/lib/api-client.ts`), store coupling removed.
 */

import { createClient } from './generated/client/client.gen'
import type { Client, ResolvedRequestOptions } from './generated/client/types.gen'
import { refresh } from './generated/sdk.gen'
import { toHeraldError } from './errors'
import type { AccessTokenHolder, SessionStore } from './session'
import type { TokenStorage } from './storage'

/** Marker header so the response interceptor only retries each request once. */
const RETRY_HEADER = 'X-Herald-Refresh-Retried'
/** Path of the refresh endpoint — never Bearer-injected, never auto-refreshed. */
const REFRESH_PATH = '/api/auth/browser-token/refresh'

export interface TransportDeps {
  baseUrl: string
  accessTokenHolder: AccessTokenHolder
  storage: TokenStorage
  session: SessionStore
}

export interface Transport {
  /** The per-instance generated client; pass to generated ops as `{ client }`. */
  readonly client: Client
}

/**
 * Run the refresh exactly once for a batch of concurrent 401s sharing this
 * client instance.
 *
 * - No stored refresh token ⇒ resolves `false` WITHOUT emitting: the caller
 *   was never (or no longer) in a restorable session, so the 401 just propagates.
 * - Refresh attempt fails (reuse / expired / family revoked) ⇒ clears the
 *   session and emits `session-expired`, then resolves `false`.
 * - Success ⇒ stores the rotated access + refresh tokens, resolves `true`.
 */
function refreshOnce(client: Client, deps: TransportDeps): Promise<boolean> {
  const rt = deps.storage.getRefreshToken()
  if (!rt) {
    // No stored refresh token → the session cannot be restored. Signal expiry
    // (design §5.4: 无 rt → 直接 session-expired) and let the 401 propagate.
    deps.session.emit({ type: 'session-expired', reason: 'refresh-failed' })
    return Promise.resolve(false)
  }

  const inFlight = (client as unknown as { __heraldRefresh?: Promise<boolean> | null })
    .__heraldRefresh
  if (inFlight) return inFlight

  const promise = (async () => {
    try {
      const { data, error } = await refresh({ client, body: { refreshToken: rt } })
      if (error || !data) {
        // 401 reuse-detected / expired / family revoked → clean re-login required.
        deps.session.emit({ type: 'session-expired', reason: 'family-revoked' })
        return false
      }
      deps.accessTokenHolder.set(data.accessToken)
      deps.storage.setRefreshToken(data.refreshToken)
      return true
    } catch {
      deps.session.emit({ type: 'session-expired', reason: 'refresh-failed' })
      return false
    } finally {
      ;(client as unknown as { __heraldRefresh?: Promise<boolean> | null }).__heraldRefresh = null
    }
  })()

  ;(client as unknown as { __heraldRefresh?: Promise<boolean> | null }).__heraldRefresh = promise
  return promise
}

/**
 * Replay the original request after a successful refresh. The generated client
 * rebuilds a fresh `Request` from the options, so we re-issue through the
 * instance client's `request` (which re-applies the now-updated Authorization).
 * We cannot return `result.response` directly: the outer pipeline would read its
 * body a second time, so we reconstruct a fresh `Response` from the parsed payload.
 */
async function replayRequest(client: Client, options: Record<string, unknown>): Promise<Response> {
  const result = (await client.request(options as never)) as {
    response?: Response
    data?: unknown
    error?: unknown
  }
  if (!result.response) {
    return new Response(null, { status: 401 })
  }
  const headers = new Headers(result.response.headers)
  const payload = result.data !== undefined ? result.data : result.error
  const body = payload !== undefined && payload !== null ? JSON.stringify(payload) : null
  return new Response(body, { status: result.response.status, headers })
}

/**
 * Create a per-instance generated client configured with `baseUrl` and the
 * Bearer + silent-refresh interceptors.
 */
export function createTransport(deps: TransportDeps): Transport {
  const client = createClient({ baseUrl: deps.baseUrl })

  // --- Request interceptor: inject Authorization: Bearer ---
  client.interceptors.request.use((request: Request, options: ResolvedRequestOptions) => {
    // The refresh endpoint authenticates with the refresh token in the body.
    if (options.url === REFRESH_PATH) {
      return request
    }
    const accessToken = deps.accessTokenHolder.get()
    if (!accessToken) {
      return request
    }
    const headers = new Headers(request.headers)
    if (!headers.has('Authorization')) {
      headers.set('Authorization', `Bearer ${accessToken}`)
    }
    return new Request(request, { headers })
  })

  // --- Response interceptor: single 401 → refresh → retry once ---
  client.interceptors.response.use(
    async (response: Response, request: Request, options: ResolvedRequestOptions) => {
      if (response.status !== 401) {
        return response
      }
      // Never auto-refresh the refresh endpoint, and never retry a request twice.
      if (options.url === REFRESH_PATH) {
        return response
      }
      if (request.headers.get(RETRY_HEADER)) {
        return response
      }

      const refreshed = await refreshOnce(client, deps)
      if (!refreshed) {
        // No token to refresh with, or refresh failed → let the 401 propagate.
        return response
      }

      // Mark the replay so a subsequent 401 is not refreshed again (loop guard).
      const replayOptions: Record<string, unknown> = { ...options }
      const headers = new Headers(
        (replayOptions.headers as Headers | Record<string, string> | undefined) ?? undefined,
      )
      headers.set(RETRY_HEADER, '1')
      replayOptions.headers = headers
      return replayRequest(client, replayOptions)
    },
  )

  return { client }
}

/**
 * Resolve a generated op result envelope to its `data`, or throw a `HeraldError`.
 * The generated client never throws with default options — network and HTTP
 * errors arrive as `{ error, response }` with `data === undefined`.
 */
export async function resolveOp<T>(
  promise: Promise<{ data?: T; error?: unknown; response?: Response }>,
): Promise<T> {
  const { data, error, response } = await promise
  if (data === undefined) {
    throw toHeraldError(error, response)
  }
  return data
}
