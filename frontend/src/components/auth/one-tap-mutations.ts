/**
 * Google One Tap login mutation (PRD `docs/prd/auth/google-one-tap.md`).
 *
 * Mirrors `src/components/auth/email-otp-mutations.ts`: TanStack `useMutation`
 * over the generated `googleOneTap` SDK, surfacing errors via the caller's
 * `onError` (which maps through `getErrorMessage` + `sonner` toast +
 * `@/paraglide/messages`).
 *
 * Herald's own login page uses the *direct-session* mode: the request body
 * omits `downstreamState`, so the backend (`google_one_tap.rs`) takes the `None`
 * branch and runs `issue_callback_token_response`, returning a flattened
 * `BrowserTokenSet` — the same shape Email-OTP verify returns. The mutation
 * therefore hands the raw `OneTapDirectResponse` to the caller's `onSuccess`;
 * the route owns token storage (`completeLoginAfterOneTap`) + navigation,
 * matching the Email-OTP boundary.
 *
 * `clientId` is the selected Herald first-party product Client App id the
 * token family is bound to — NOT the Google
 * `client_id` used to initialize GIS (that comes from `publicConfig` and is
 * consumed inside `one-tap-login.tsx`).
 *
 * Type note: the generated OpenAPI `GoogleOneTapResponses.200` is typed as
 * `OneTapCodeResponse` (the downstream-code branch). The direct-session branch
 * we use here actually returns `OneTapDirectResponse` at runtime; the utoipa
 * schema only models one 200 shape. We assert accordingly — this does not paper
 * over a runtime risk, since the absence of `downstreamState` deterministically
 * selects the direct branch.
 */
import { useMutation } from '@tanstack/react-query'
import { googleOneTap } from '@/lib/api-generated'
import type { OneTapDirectResponse } from '@/lib/api-generated'
export interface UseOneTapLoginMutationOptions {
  realmId: string
  clientId: string
  onSuccess?: (tokenResponse: OneTapDirectResponse) => void
  onError?: (error: unknown) => void
}

export function useOneTapLoginMutation({
  realmId,
  clientId,
  onSuccess,
  onError,
}: UseOneTapLoginMutationOptions) {
  return useMutation({
    mutationFn: async (payload: { credential: string }): Promise<OneTapDirectResponse> => {
      const response = await googleOneTap({
        path: { realmId },
        body: {
          credential: payload.credential,
          clientId,
        },
      })
      if (response.error) throw response.error
      return response.data as unknown as OneTapDirectResponse
    },
    onSuccess: (tokenResponse) => {
      onSuccess?.(tokenResponse)
    },
    onError: (error) => {
      onError?.(error)
    },
  })
}
