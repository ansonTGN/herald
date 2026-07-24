/**
 * Google One Tap entry for the Herald login page (design §4.4.3).
 *
 * Renders the GIS prompt overlay (`google.accounts.id.prompt`) — the prompt
 * itself is positioned and shown by Google, so this component emits only a
 * zero-size anchor. Lifecycle:
 *
 *   1. `useScript` lazily injects `https://accounts.google.com/gsi/client`.
 *   2. On 'ready', call `google.accounts.id.initialize({ client_id, callback })`
 *      then `google.accounts.id.prompt()`. `initialize` may be called at most
 *      once per page load; a `promptedRef` guard keeps the effect idempotent
 *      across React re-renders.
 *   3. When the user picks an account, Google invokes `callback` with the ID
 *      Token; we POST it to `/api/oauth/{realmId}/google/one-tap` (direct-
 *      session mode — no `downstreamState`) and hand the resulting
 *      `OneTapDirectResponse` to `onSuccess`.
 *
 * Degradation is silent and expected (PRD §7): if the script fails to load, or
 * Google decides not to show the prompt (no signed-in Google account, browser
 * cooldown, origin not authorized), this entry reports `onUnavailable` and the
 * parent hides it — the password/OAuth buttons are unaffected. Such cases are
 * not errors and are never toasted.
 *
 * The visible prompt is fully controlled by Google (PRD §7). We do not render a
 * fallback button; users who miss the prompt fall back to the existing
 * password / OAuth buttons on the page.
 */
import { useCallback, useEffect, useRef } from 'react'
import { toast } from 'sonner'
import { useScript } from '@/hooks/use-script'
import { useOneTapLoginMutation } from '@/components/auth/one-tap-mutations'
import { getErrorMessage } from '@/lib/error-utils'
import { m } from '@/paraglide/messages'
import type { OneTapDirectResponse } from '@/lib/api-generated'

const GIS_SCRIPT_SRC = 'https://accounts.google.com/gsi/client'

export interface OneTapLoginProps {
  realmId: string
  /** Google OAuth client_id for this realm (from `publicConfig.oauthProviders`). */
  googleClientId: string
  /** Invoked once the backend issues the direct-session token set. */
  onSuccess: (tokenResponse: OneTapDirectResponse) => void
  /**
   * Invoked when One Tap cannot be offered (script load failure). The parent
   * should hide this entry; the user keeps the password/OAuth options.
   */
  onUnavailable?: () => void
}

export function OneTapLogin({
  realmId,
  googleClientId,
  onSuccess,
  onUnavailable,
}: OneTapLoginProps) {
  const status = useScript(GIS_SCRIPT_SRC)
  // `initialize` must run at most once per page load; guard against effect
  // re-runs (React strict mode, dep changes).
  const promptedRef = useRef(false)

  const oneTapMutation = useOneTapLoginMutation({
    realmId,
    onSuccess: (tokenResponse) => {
      onSuccess(tokenResponse)
    },
    onError: (error) => {
      const message = getErrorMessage(error)
      toast.error(`${m['auth.login.one_tap_failed']()}: ${message}`)
    },
  })

  // Hold the latest mutate fn in a ref so the callback handed to
  // `google.accounts.id.initialize` is referentially stable (initialize can
  // only run once per page load) yet always calls the freshest mutation.
  const mutateRef = useRef(oneTapMutation.mutate)
  useEffect(() => {
    mutateRef.current = oneTapMutation.mutate
  })

  const handleCredential = useCallback((response: GoogleCredentialResponse) => {
    mutateRef.current({ credential: response.credential })
  }, [])

  useEffect(() => {
    if (status !== 'ready') return
    if (promptedRef.current) return
    const google = window.google
    if (!google) {
      // Script reported ready but `window.google` is absent — treat as
      // unsupported and degrade silently.
      onUnavailable?.()
      return
    }
    promptedRef.current = true
    google.accounts.id.initialize({
      client_id: googleClientId,
      callback: handleCredential,
    })
    google.accounts.id.prompt()
    // `googleClientId` is captured here on purpose: changing it after init has
    // no supported recovery (initialize is once-per-load), so we do not want
    // the effect to re-run. `handleCredential` is stable (empty deps).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status])

  if (status === 'error') {
    // Script failed to load — report once and render nothing. Delegated to a
    // child that fires the callback a single time, since the parent may render
    // through this branch more than once before unmounting.
    return <UnavailableReporter onUnavailable={onUnavailable} />
  }

  // The prompt overlay is rendered by Google into its own DOM; this anchor only
  // needs to exist as a mount point and for test selection.
  return <div data-testid="one-tap-container" aria-hidden="true" />
}

/**
 * Fires `onUnavailable` once on mount. Extracted so the main component can stay
 * declarative: the 'error' status is terminal, but React may re-render through
 * this branch, and we only want the callback to run a single time.
 */
function UnavailableReporter({ onUnavailable }: { onUnavailable?: () => void }) {
  const firedRef = useRef(false)
  useEffect(() => {
    if (firedRef.current) return
    firedRef.current = true
    onUnavailable?.()
  }, [onUnavailable])
  return null
}
