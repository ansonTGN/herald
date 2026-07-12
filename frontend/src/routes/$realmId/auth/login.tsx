import { createFileRoute, useRouter } from '@tanstack/react-router'
import { useForm } from '@tanstack/react-form'
import { useMutation, useQuery } from '@tanstack/react-query'
import type {
  LoginRequestPayload,
  VerifyTotpResponse,
  PasskeyVerifyResponse,
  LegalAgreementSummary,
  AuthConsentAgreement,
} from '@/lib/api-generated'
import { loginSchema } from '@/lib/schemas/common'
import { loginSearchSchema, type LoginSearchParams } from '@/lib/schemas/search-params'
import { getErrorMessage, getFieldErrorMessage } from '@/lib/error-utils'
import {
  loginFlow,
  completeLoginAfterTotp,
  completeLoginAfterPasskey,
  isConsentRequired,
  getSafeRedirect,
  checkAdminPermission,
  validateOAuthParams,
} from '@/lib/auth-utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { resolveBrandName } from '@/lib/white-label-brand'
import { TotpVerificationForm } from '@/components/auth/totp-verification-form'
import { PasskeyLoginForm } from '@/components/auth/passkey-login-form'
import { Passkey2FaForm } from '@/components/auth/passkey-2fa-form'
import { TurnstileWidget } from '@/components/auth/turnstile-widget'
import {
  publicConfigQueryOptions,
  toAuthConsentAgreements,
  turnstileStatusQueryOptions,
} from '@/data/query-options'
import { Link } from '@tanstack/react-router'
import { useOAuthLogin } from '@/hooks/use-oauth-login'
import { useState } from 'react'
import { toast } from 'sonner'
import { m } from '@/paraglide/messages'
import { AgreementLinks } from '@/components/legal/AgreementLinks'
import { formatDate } from '@/lib/date-utils'
import {
  realmPath,
  useCurrentSearch,
  useOptionalRouteParams,
  useResolvedRealmContext,
} from '@/lib/realm-routing'

// react-hooks/immutability forbids assigning window.location.href inside
// callbacks passed to hooks; route it through a module-level helper.
function navigateExternally(url: string): void {
  window.location.href = url
}

interface TotpStep {
  tempToken: string
}

interface ConsentStep {
  agreements: LegalAgreementSummary[]
  originalPayload: LoginRequestPayload
}

/**
 * Passkey second-factor step. Reached when a password login returns
 * `secondFactors` containing `"passkey"`. Carries the temp token plus the full
 * `secondFactors` list so the form can show the TOTP fallback link only when
 * `"totp"` is also present.
 */
interface PasskeySecondFactorStep {
  tempToken: string
  secondFactors: string[]
}

const DEFAULT_CLIENT_ID = 'admin-web-console'

/**
 * Whether the realm exposes a passkey login entry point for the current
 * browser. Lazily flipped to false when the begin-options call 404s (realm
 * passkey disabled) or the browser lacks WebAuthn, so the entry is hidden
 * without affecting the password form.
 */

export const Route = createFileRoute('/$realmId/auth/login')({
  component: LoginPage,
  validateSearch: (search) => loginSearchSchema.parse(search),
  // NOTE: Authentication and redirect logic is handled by __root.tsx
  // This allows us to use cached auth data and avoid redundant API calls
})

export function LoginPage() {
  const router = useRouter()
  const routeParams = useOptionalRouteParams<{ realmId?: string }>(Route)
  const resolvedRealmContext = useResolvedRealmContext()
  const realmContext = routeParams.realmId
    ? { ...resolvedRealmContext, realmId: routeParams.realmId, isCustomDomain: false }
    : resolvedRealmContext
  const { realmId } = realmContext
  const search = loginSearchSchema.parse(useCurrentSearch()) as LoginSearchParams
  const { initiateOAuthLogin } = useOAuthLogin()

  const [totpStep, setTotpStep] = useState<TotpStep | null>(null)
  const [consentStep, setConsentStep] = useState<ConsentStep | null>(null)
  const [globalError, setGlobalError] = useState<string | null>(null)
  // Passkey second-factor step (reached when a password login returns
  // secondFactors containing "passkey"). The first-factor entry point does
  // not need a dedicated step — PasskeyLoginForm manages its own conditional
  // UI lifecycle and is mounted alongside the password form.
  const [passkeySecondFactor, setPasskeySecondFactor] = useState<PasskeySecondFactorStep | null>(
    null
  )
  // Tracks whether the realm exposes a passkey login entry for this browser.
  // Defaults to true (WebAuthn-capable) and is flipped to false when the
  // begin-options call 404s or the browser is unsupported, hiding the entry.
  const [passkeyAvailable, setPasskeyAvailable] = useState(true)

  const { data: publicConfig, isLoading } = useQuery(publicConfigQueryOptions(realmId))
  const { data: turnstileStatus, isLoading: loadingTurnstile } = useQuery(
    turnstileStatusQueryOptions(realmId)
  )

  // Per-realm white-label config (FE-D02/FE-D03). Derived once so every auth
  // sub-state (consent, TOTP, passkey 2FA, main form) reuses the same brand
  // presentation — missing one would silently drop the brand (design §6.3 risk).
  const whiteLabel = publicConfig?.whiteLabel ?? null

  const oauthProviders = publicConfig?.oauthProviders ?? []
  const isRegistrationAllowed = publicConfig?.registration?.enabled === true

  const { oauthParams, hasPartialOAuth } = validateOAuthParams(search)

  const loginMutation = useMutation({
    mutationFn: async (values: {
      username: string
      password: string
      agreements?: AuthConsentAgreement[]
      turnstileToken?: string
    }) => {
      const isEmail = values.username.includes('@')
      const clientId = search.clientId || DEFAULT_CLIENT_ID

      const loginData: LoginRequestPayload = {
        clientId,
        email: isEmail ? values.username : undefined,
        username: isEmail ? undefined : values.username,
        password: values.password,
        turnstileToken: values.turnstileToken || null,
        ...(values.agreements ? { agreements: values.agreements } : {}),
        ...(oauthParams ?? {}),
      }

      const result = await loginFlow(realmId, loginData)
      return { result, payload: loginData }
    },
    onSuccess: async (data) => {
      setGlobalError(null)
      setConsentStep(null)
      const { response } = data.result

      // --- Second-factor routing (design §5.3, backward compatible) ----------
      // Read order: prefer `secondFactors` when present and non-empty; only
      // when it is ABSENT do we fall back to the legacy `requiresTotp` path.
      // This keeps the existing password+TOTP login 100% unchanged for any
      // backend that does not yet return `secondFactors`.
      const secondFactors =
        response.secondFactors && response.secondFactors.length > 0 ? response.secondFactors : null

      if (secondFactors) {
        if (!response.tempToken) {
          // Defensive: secondFactors without a tempToken cannot proceed to any
          // 2FA form. Fall through to consent / direct-login handling below.
        } else if (secondFactors.includes('passkey')) {
          // Passkey-capable users (optionally alongside TOTP) land on the
          // Passkey second-factor form, which offers a TOTP fallback when the
          // list also contains "totp".
          setPasskeySecondFactor({ tempToken: response.tempToken, secondFactors })
          return
        } else {
          // secondFactors present but has no passkey → TOTP path (covers
          // ["totp"] and any unknown factors gracefully degrading to TOTP).
          setTotpStep({ tempToken: response.tempToken })
          return
        }
      } else if (response.requiresTotp && response.tempToken) {
        // Legacy fallback (unchanged behaviour): backend without secondFactors.
        setTotpStep({ tempToken: response.tempToken })
        setConsentStep(null)
        return
      }

      if (isConsentRequired(response)) {
        const agreements = response.agreements ?? []
        if (agreements.length > 0) {
          setConsentStep({ agreements, originalPayload: data.payload })
          return
        }
      }

      if (response.redirectTo) {
        navigateExternally(response.redirectTo)
        return
      }

      toast.success(m['auth.login.login_successful']())

      const userRealmId = response.realmId || realmId
      let redirectPath = search.redirect || data.result.redirectPath

      // Prevent open redirect attacks
      redirectPath = getSafeRedirect(redirectPath)

      if (redirectPath === '/') {
        redirectPath = checkAdminPermission() ? '/manage' : '/user/profile'
      }

      if (redirectPath.startsWith('http://') || redirectPath.startsWith('https://')) {
        navigateExternally(redirectPath)
        return
      }

      await router.navigate({
        to: realmPath({ ...realmContext, realmId: userRealmId }, redirectPath),
        params: { realmId: userRealmId },
      })
    },
  })

  const form = useForm({
    defaultValues: { username: '', password: '', turnstileToken: '' },
    onSubmit: async ({ value }) => {
      setGlobalError(null)
      if (hasPartialOAuth) return
      loginMutation.mutate(
        {
          username: value.username,
          password: value.password,
          turnstileToken: value.turnstileToken || undefined,
        },
        {
          onError: (error: unknown) => {
            const message = getErrorMessage(error)
            toast.error(message)
            setGlobalError(message)
          },
        }
      )
    },
  })

  async function handleConsentAgree() {
    if (!consentStep) return
    setGlobalError(null)

    const agreements = toAuthConsentAgreements(consentStep.agreements)
    const username = form.getFieldValue('username')
    const password = form.getFieldValue('password')
    const turnstileToken = form.getFieldValue('turnstileToken') || undefined

    loginMutation.mutate(
      { username, password, agreements, turnstileToken },
      {
        onError: (error: unknown) => {
          const message = getErrorMessage(error)
          toast.error(message)
          setGlobalError(message)
        },
      }
    )
  }

  function handleConsentDecline() {
    setConsentStep(null)
    setGlobalError(null)
  }

  async function handleTotpSuccess(verifyResponse: VerifyTotpResponse): Promise<void> {
    toast.success(m['auth.login.login_successful']())

    const { redirectPath, redirectTo } = await completeLoginAfterTotp(realmId, verifyResponse)

    if (redirectTo) {
      navigateExternally(redirectTo)
      return
    }

    // Prevent open redirect attacks
    let safeRedirectPath = getSafeRedirect(search.redirect, redirectPath)

    if (safeRedirectPath === '/') {
      safeRedirectPath = checkAdminPermission() ? '/manage' : '/user/profile'
    }

    if (safeRedirectPath.startsWith('http://') || safeRedirectPath.startsWith('https://')) {
      navigateExternally(safeRedirectPath)
      return
    }

    await router.navigate({
      to: realmPath(realmContext, safeRedirectPath),
      params: { realmId },
    })
  }

  /**
   * Shared completion handler for a Passkey login that has already passed the
   * consent interlock (handled inside the passkey forms). Behaviour mirrors
   * `handleTotpSuccess`: fetch auth data, store it, redirect safely. Used by
   * both the first-factor form and the second-factor form.
   */
  async function handlePasskeySuccess(verifyResponse: PasskeyVerifyResponse): Promise<void> {
    toast.success(m['auth.login.login_successful']())

    const { redirectPath, redirectTo } = await completeLoginAfterPasskey(realmId, verifyResponse)

    if (redirectTo) {
      navigateExternally(redirectTo)
      return
    }

    // Prevent open redirect attacks
    let safeRedirectPath = getSafeRedirect(search.redirect, redirectPath)

    if (safeRedirectPath === '/') {
      safeRedirectPath = checkAdminPermission() ? '/manage' : '/user/profile'
    }

    if (safeRedirectPath.startsWith('http://') || safeRedirectPath.startsWith('https://')) {
      navigateExternally(safeRedirectPath)
      return
    }

    await router.navigate({
      to: realmPath(realmContext, safeRedirectPath),
      params: { realmId },
    })
  }

  if (consentStep) {
    return (
      <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
        <Card className="w-full max-w-md" data-testid="login-reconsent-view">
          <CardHeader className="text-center">
            <CardTitle data-testid="login-reconsent-title">
              {m['auth.login.reconsent_title']()}
            </CardTitle>
            <p className="text-sm text-muted-foreground">
              {m['auth.login.reconsent_description']()}
            </p>
          </CardHeader>
          <CardContent className="space-y-4">
            {consentStep.agreements.map((agreement) => (
              <div
                key={agreement.version_id}
                className="rounded border p-3"
                data-testid={`login-reconsent-agreement-${agreement.agreement_type}`}
              >
                <div className="font-medium">
                  <AgreementLinks
                    realmId={realmId}
                    agreements={[agreement]}
                    agreementType={
                      agreement.agreement_type as 'terms_of_service' | 'privacy_policy'
                    }
                  />
                </div>
                <div
                  className="text-sm text-muted-foreground"
                  data-testid={`login-reconsent-agreement-${agreement.agreement_type}-version`}
                >
                  {m['legal.version_label']()}: {agreement.version_no} •{' '}
                  {m['legal.effective_date_label']()}: {formatDate(agreement.effective_at)}
                </div>
              </div>
            ))}
            <Button
              type="button"
              disabled={loginMutation.isPending}
              className="w-full"
              data-testid="login-agree-and-continue-button"
              onClick={handleConsentAgree}
            >
              {loginMutation.isPending
                ? m['auth.login.logging_in']()
                : m['auth.login.agree_and_continue']()}
            </Button>
            <Button
              type="button"
              variant="outline"
              className="w-full"
              data-testid="login-decline-back-button"
              onClick={handleConsentDecline}
            >
              {m['auth.login.decline_back_to_login']()}
            </Button>
          </CardContent>
        </Card>
      </AuthPageWrapper>
    )
  }

  if (totpStep) {
    return (
      <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
        <TotpVerificationForm
          realmId={realmId}
          tempToken={totpStep.tempToken}
          onSuccess={handleTotpSuccess}
          onBack={() => setTotpStep(null)}
        />
      </AuthPageWrapper>
    )
  }

  if (passkeySecondFactor) {
    return (
      <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
        <Passkey2FaForm
          realmId={realmId}
          tempToken={passkeySecondFactor.tempToken}
          secondFactors={passkeySecondFactor.secondFactors}
          onSuccess={handlePasskeySuccess}
          onBack={() => setPasskeySecondFactor(null)}
          // Only offer the TOTP fallback when the user actually has TOTP.
          onSwitchToTotp={
            passkeySecondFactor.secondFactors.includes('totp')
              ? () => {
                  setTotpStep({ tempToken: passkeySecondFactor.tempToken })
                  setPasskeySecondFactor(null)
                }
              : undefined
          }
        />
      </AuthPageWrapper>
    )
  }

  return (
    <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
      <Card className="w-full max-w-md" data-testid="login-card">
        <CardHeader className="text-center">
          <CardTitle data-testid="login-title" className="text-2xl">
            {whiteLabel?.loginTitle ?? resolveBrandName(whiteLabel, publicConfig?.realmName)}
          </CardTitle>
          <p className="text-sm text-muted-foreground">
            {whiteLabel?.loginSubtitle ??
              publicConfig?.realmDescription ??
              m['auth.login.login_to_account']()}
          </p>
        </CardHeader>
        <CardContent>
          {globalError && (
            <div
              className="mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-600 text-sm"
              data-testid="login-error-message"
            >
              {globalError}
            </div>
          )}

          {hasPartialOAuth && (
            <div
              className="mb-4 p-3 bg-red-50 border border-red-200 rounded text-red-600 text-sm"
              data-testid="oauth-incomplete-error"
            >
              {m['auth.oauth_params_incomplete']()}
            </div>
          )}

          <form
            onSubmit={(e) => {
              e.preventDefault()
              form.handleSubmit()
            }}
            className="space-y-4"
            data-testid="login-form"
          >
            <form.Field name="username" validators={{ onChange: loginSchema.shape.username }}>
              {(field) => (
                <div>
                  <Label htmlFor="username">{m['auth.login.username_or_email']()}</Label>
                  <Input
                    id="username"
                    type="text"
                    value={field.state.value}
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value)}
                    disabled={loginMutation.isPending}
                    data-testid="email-input"
                  />
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-red-500 mt-1">
                      {getFieldErrorMessage(field.state.meta.errors[0])}
                    </p>
                  )}
                </div>
              )}
            </form.Field>

            <form.Field name="password" validators={{ onChange: loginSchema.shape.password }}>
              {(field) => (
                <div>
                  <div className="flex items-center justify-between">
                    <Label htmlFor="password">{m['auth.login.password']()}</Label>
                    <Link
                      to={realmPath(realmContext, '/auth/forgot-password')}
                      className="text-sm font-medium text-primary hover:text-primary/80"
                      data-testid="forgot-password-link"
                    >
                      {m['auth.forgot_password.forgot_link']()}
                    </Link>
                  </div>
                  <Input
                    id="password"
                    type="password"
                    value={field.state.value}
                    onBlur={field.handleBlur}
                    onChange={(e) => field.handleChange(e.target.value)}
                    disabled={loginMutation.isPending}
                    data-testid="password-input"
                  />
                  {field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-red-500 mt-1">
                      {getFieldErrorMessage(field.state.meta.errors[0])}
                    </p>
                  )}
                </div>
              )}
            </form.Field>

            {!loadingTurnstile && turnstileStatus?.enabled && (
              <form.Field name="turnstileToken">
                {(field) => (
                  <TurnstileWidget
                    siteKey={turnstileStatus.site_key || ''}
                    onTokenChange={(token) => field.handleChange(token || '')}
                    onError={(error) => console.error('Turnstile error:', error)}
                  />
                )}
              </form.Field>
            )}

            <Button
              type="submit"
              disabled={loginMutation.isPending || hasPartialOAuth}
              className="w-full"
              data-testid="login-submit-button"
            >
              {loginMutation.isPending ? m['auth.login.logging_in']() : m['auth.login.submit']()}
            </Button>

            <div
              className="text-center text-sm text-muted-foreground pt-1"
              data-testid="login-consent-statement"
            >
              {m['auth.login.consent_statement']()}
              <AgreementLinks
                realmId={realmId}
                beforeText=" "
                linkClassName="text-primary hover:text-primary/80 underline underline-offset-2"
              />
            </div>
          </form>

          {/* Passkey first-factor entry. Mounted whenever the realm exposes
              passkey for this browser (passkeyAvailable). The form fetches the
              begin-challenge on mount and arms the conditional (autofill) UI;
              if the realm has passkey disabled (options 404) or the browser is
              unsupported it calls onUnavailable and we hide the entry without
              touching the password form. */}
          {passkeyAvailable && (
            <div className="mt-4">
              <PasskeyLoginForm
                realmId={realmId}
                clientId={search.clientId || DEFAULT_CLIENT_ID}
                turnstileToken={form.getFieldValue('turnstileToken') || undefined}
                oauth={
                  oauthParams
                    ? {
                        clientId: oauthParams.oauthClientId,
                        redirectUri: oauthParams.redirectUri,
                        state: oauthParams.state,
                      }
                    : null
                }
                onSuccess={handlePasskeySuccess}
                onUnavailable={() => setPasskeyAvailable(false)}
              />
            </div>
          )}

          {!isLoading && oauthProviders.length > 0 && (
            <div className="space-y-3 mt-6">
              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-background px-2 text-muted-foreground">
                    {m['auth.login.or_continue_with']()}
                  </span>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                {oauthProviders.map((provider) => (
                  <Button
                    key={provider.name}
                    variant="outline"
                    onClick={() => initiateOAuthLogin(realmId, provider.name, oauthParams?.state)}
                    disabled={loginMutation.isPending}
                    data-testid={`oauth-login-button-${provider.name}`}
                  >
                    {provider.displayName}
                  </Button>
                ))}
              </div>
            </div>
          )}

          {isRegistrationAllowed && (
            <div className="mt-4 text-center">
              <span className="text-sm text-gray-500">{m['auth.login.no_account']()} </span>
              <Link
                to={realmPath(realmContext, '/auth/register')}
                className="text-sm font-medium text-primary hover:text-primary/80"
                data-testid="register-link"
              >
                {m['auth.login.register_link']()}
              </Link>
            </div>
          )}
        </CardContent>
      </Card>
    </AuthPageWrapper>
  )
}
