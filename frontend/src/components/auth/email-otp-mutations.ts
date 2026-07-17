/**
 * Email-OTP login send/verify mutations (design §4.1, §4.2, §4.4.2).
 *
 * Mirrors the pattern in `src/components/oauth-config/oauth-mutations.ts`:
 * TanStack `useMutation` over the generated SDK, with errors mapped through
 * `getErrorMessage` (`src/lib/error-utils`) and surfaced via `sonner` toasts +
 * `@/paraglide/messages` (`m`).
 *
 * Send special-cases the 409 body: OTP send returns
 * `EmailOtpConflictResponse` (`{ code, consentRequired?, agreements?, message }`)
 * on (a) an unregistered email when auto-register is ON but consent is missing,
 * and (b) an unregistered email when auto-register is OFF. Those are NOT
 * user-facing "errors" — they are control-flow signals the form must render
 * against (consent gate / not-registered guidance, design §4.4.2). The send
 * mutation therefore exposes them via the returned `EmailOtpSendError` shape
 * instead of throwing, so the component can branch without a try/catch.
 */

import { useMutation } from '@tanstack/react-query'
import { toast } from 'sonner'
import { send, verify } from '@/lib/api-generated'
import type {
  BrowserTokenResponse,
  EmailOtpConflictResponse,
  EmailOtpSendRequest,
  EmailOtpSendResponse,
  EmailOtpVerifyRequest,
  LegalAgreementSummary,
} from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

/**
 * Conflict outcome surfaced to the form when send returns 409.
 *
 * - `code: 'consent_required'` → the form renders the `agreements` list and
 *   re-sends with the accepted agreements (built via `toAuthConsentAgreements`).
 * - `code: 'email_not_registered'` → the form shows the localized guidance
 *   message and the explicit-register link.
 */
export interface EmailOtpSendConflict {
  code: string
  consentRequired?: boolean | null
  agreements?: LegalAgreementSummary[] | null
  message: string
}

/**
 * Successful send result. On conflict, `conflict` is set and `data` is null
 * (no code was sent). On other errors the mutation throws and `onError` runs.
 */
export interface EmailOtpSendResult {
  data: EmailOtpSendResponse | null
  conflict: EmailOtpSendConflict | null
}

/**
 * Detect an `EmailOtpConflictResponse` body on a send error. The generated
 * hey-api axios client (default `throwOnError: false`) exposes the parsed HTTP
 * body on `response.error` directly, so the conflict body arrives here as
 * `{ code, consentRequired?, agreements?, message }` (design §4.2.2). The only
 * two conflict codes are `consent_required` and `email_not_registered`
 * (backend `email_otp.rs`); any other code/value means a generic error and the
 * mutation throws so the form renders the error region.
 */
const OTP_CONFLICT_CODES = new Set(['consent_required', 'email_not_registered'])

function extractConflict(error: unknown): EmailOtpConflictResponse | null {
  if (!error || typeof error !== 'object') return null
  const record = error as Partial<EmailOtpConflictResponse>
  if (
    typeof record.code === 'string' &&
    OTP_CONFLICT_CODES.has(record.code) &&
    typeof record.message === 'string'
  ) {
    return record as EmailOtpConflictResponse
  }
  return null
}

export interface UseEmailOtpSendMutationOptions {
  realmId: string
  onSuccess?: (result: EmailOtpSendResult) => void
  onError?: (error: unknown) => void
}

/**
 * Send mutation. Surfaces a 409 conflict via the returned `EmailOtpSendResult`
 * (data=null, conflict set) — it does NOT throw on 409, so the form can branch
 * on `code`/`agreements`/`message` without try/catch. All other errors throw
 * normally and flow to `onError`.
 */
export function useEmailOtpSendMutation({
  realmId,
  onSuccess,
  onError,
}: UseEmailOtpSendMutationOptions) {
  return useMutation({
    mutationFn: async (payload: {
      email: string
      clientId: string
      turnstileToken?: string | null
      agreements?: Array<{ agreementType: string; versionId: string }>
    }): Promise<EmailOtpSendResult> => {
      const body: EmailOtpSendRequest = {
        clientId: payload.clientId,
        email: payload.email,
        ...(payload.turnstileToken ? { turnstileToken: payload.turnstileToken } : {}),
        ...(payload.agreements ? { agreements: payload.agreements } : {}),
      }
      const response = await send({ path: { realmId }, body })
      if (response.error) {
        const conflict = extractConflict(response.error)
        if (conflict) {
          return {
            data: null,
            conflict: {
              code: conflict.code,
              consentRequired: conflict.consentRequired ?? null,
              agreements: conflict.agreements ?? null,
              message: conflict.message,
            },
          }
        }
        throw response.error
      }
      return { data: response.data as EmailOtpSendResponse, conflict: null }
    },
    onSuccess: (result) => {
      // Only toast on a real send (not on conflict). Conflict handling is the
      // form's job (render gate / guidance), not a toast.
      if (result.data) {
        toast.success(m['auth.email_otp.send_success']())
      }
      onSuccess?.(result)
    },
    onError: (error) => {
      onError?.(error)
    },
  })
}

export interface UseEmailOtpVerifyMutationOptions {
  realmId: string
  onSuccess?: (tokenResponse: BrowserTokenResponse) => void
  onError?: (error: unknown) => void
}

/**
 * Verify mutation. On 200 returns the raw `BrowserTokenResponse` to the caller
 * (the route owns `completeLoginAfterEmailOtp` + navigation — design §4.1
 * boundary; mirror `PasskeyLoginForm`/`handlePasskeySuccess`). Verify never
 * returns `redirectTo`/PKCE, so no exchange branch is needed here.
 */
export function useEmailOtpVerifyMutation({
  realmId,
  onSuccess,
  onError,
}: UseEmailOtpVerifyMutationOptions) {
  return useMutation({
    mutationFn: async (payload: {
      email: string
      code: string
      clientId: string
      agreements?: Array<{ agreementType: string; versionId: string }>
    }): Promise<BrowserTokenResponse> => {
      const body: EmailOtpVerifyRequest = {
        clientId: payload.clientId,
        email: payload.email,
        code: payload.code,
        ...(payload.agreements ? { agreements: payload.agreements } : {}),
      }
      const response = await verify({ path: { realmId }, body })
      if (response.error) throw response.error
      return response.data as BrowserTokenResponse
    },
    onSuccess: (tokenResponse) => {
      onSuccess?.(tokenResponse)
    },
    onError: (error) => {
      onError?.(error)
    },
  })
}
