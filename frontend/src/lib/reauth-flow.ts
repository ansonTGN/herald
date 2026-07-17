/**
 * Unified re-authentication (reauth) flow helper.
 *
 * High-risk user operations (design §4.2.2 REAUTH + §4.4.2) require a
 * single-use, short-lived (~120s) `reauthToken` bound to a target operation.
 * The flow is:
 *
 *   1. `handleBeginReauth({ targetOperation })` → `availableFactors` (+ challenge)
 *   2. `handleVerifyReauth({ targetOperation, factor, password })` → `reauthToken`
 *   3. Pass the resulting `reauthToken` in the protected call's body.
 *
 * The token is single-use and short-lived, so it must be obtained immediately
 * before the protected call (not ahead of time). The Bearer interceptor in
 * `api-client.ts` injects the access token and handles silent 401 refresh, so
 * these calls are authenticated automatically.
 *
 * Per AGENTS.md Rule 2 (simplicity first), the default/minimal factor is the
 * universally-available `password` factor. Each high-risk form already collects
 * the user's current password, which is reused here as the reauth credential.
 *
 * Documented errors (design §4.2.2): 401 = invalid factor/credential;
 * 409 = expired / already consumed / target-operation mismatch (re-prompt).
 */
import { handleBeginReauth, handleVerifyReauth } from '@/lib/api-generated'
import type { TargetOperation } from '@/lib/api-generated'
import { resolveApiError } from '@/lib/error-utils'
import { m } from '@/paraglide/messages'

/** Error thrown when re-authentication fails (bad password, expired, etc.). */
export class ReauthError extends Error {
  /** HTTP status from the failing reauth call, when available. */
  readonly status?: number

  constructor(message: string, status?: number) {
    super(message)
    this.name = 'ReauthError'
    this.status = status
  }
}

/**
 * Maps a reauth API failure to a user-facing message. Per the design:
 * - 401 → wrong password / invalid factor
 * - 409 → expired / consumed / target mismatch (ask to retry)
 * - other → generic reauth failed
 */
function reauthFailureMessage(status: number | undefined): string {
  switch (status) {
    case 401:
      return m['reauth.wrong_password']()
    case 409:
      return m['reauth.expired']()
    default:
      return m['reauth.failed']()
  }
}

/**
 * Obtain a single-use `reauthToken` for a high-risk operation using the
 * **password** factor.
 *
 * Runs `handleBeginReauth` → `handleVerifyReauth` and returns the token. Throws
 * a {@link ReauthError} (carrying the HTTP status) on any failure so the caller
 * can surface the appropriate message and let the user retry.
 *
 * @param targetOperation - The operation the token will be bound to.
 * @param password - The user's current password (reused as the reauth factor).
 * @returns The short-lived, single-use `reauthToken`.
 */
export async function obtainReauthToken(
  targetOperation: TargetOperation,
  password: string
): Promise<string> {
  // Step 1: begin. The available factors are returned but the Herald console
  // always uses the password factor for the minimal viable flow.
  const beginResponse = await handleBeginReauth({ body: { targetOperation } })
  if (beginResponse.error) {
    const { status } = resolveApiError(beginResponse.error)
    throw new ReauthError(reauthFailureMessage(status), status)
  }
  const availableFactors = beginResponse.data?.availableFactors ?? []
  if (!availableFactors.includes('password')) {
    // The password factor is not available for this user/op — cannot proceed
    // with the minimal flow. Surface a generic failure.
    throw new ReauthError(reauthFailureMessage(undefined), undefined)
  }

  // Step 2: verify with the password factor → obtain the single-use token.
  const verifyResponse = await handleVerifyReauth({
    body: {
      targetOperation,
      factor: 'password',
      password,
    },
  })
  if (verifyResponse.error) {
    const { status } = resolveApiError(verifyResponse.error)
    throw new ReauthError(reauthFailureMessage(status), status)
  }
  const reauthToken = verifyResponse.data?.reauthToken
  if (!reauthToken) {
    throw new ReauthError(reauthFailureMessage(undefined), undefined)
  }
  return reauthToken
}
