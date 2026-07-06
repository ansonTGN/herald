import { createFileRoute, useRouter } from '@tanstack/react-router'
import { useForm } from '@tanstack/react-form'
import { useMutation, useQuery } from '@tanstack/react-query'
import type {
  LoginRequestPayload,
  VerifyTotpResponse,
  LegalAgreementSummary,
  AuthConsentAgreement,
} from '@/lib/api-generated'
import { loginSchema } from '@/lib/schemas/common'
import { loginSearchSchema, type LoginSearchParams } from '@/lib/schemas/search-params'
import { getErrorMessage, getFieldErrorMessage } from '@/lib/error-utils'
import {
  loginFlow,
  completeLoginAfterTotp,
  getSafeRedirect,
  checkAdminPermission,
  validateOAuthParams,
} from '@/lib/auth-utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { TotpVerificationForm } from '@/components/auth/totp-verification-form'
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

interface TotpStep {
  tempToken: string
}

interface ConsentStep {
  agreements: LegalAgreementSummary[]
  originalPayload: LoginRequestPayload
}

const DEFAULT_CLIENT_ID = 'admin-web-console'

function isConsentRequired(response: {
  consentRequired?: boolean | null
  consent_required?: boolean | null
}): boolean {
  return (
    !!response.consentRequired ||
    !!(response as { consent_required?: boolean | null }).consent_required
  )
}

export const Route = createFileRoute('/$realmId/auth/login')({
  component: LoginPage,
  validateSearch: (search) => loginSearchSchema.parse(search),
  // NOTE: Authentication and redirect logic is handled by __root.tsx
  // This allows us to use cached auth data and avoid redundant API calls
})

export function LoginPage() {
  const router = useRouter()
  const { realmId } = Route.useParams()
  const search = Route.useSearch() as LoginSearchParams
  const { initiateOAuthLogin } = useOAuthLogin()

  const [totpStep, setTotpStep] = useState<TotpStep | null>(null)
  const [consentStep, setConsentStep] = useState<ConsentStep | null>(null)
  const [globalError, setGlobalError] = useState<string | null>(null)

  const { data: publicConfig, isLoading } = useQuery(publicConfigQueryOptions(realmId))
  const { data: turnstileStatus, isLoading: loadingTurnstile } = useQuery(
    turnstileStatusQueryOptions(realmId)
  )

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

      if (response.requiresTotp && response.tempToken) {
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
        window.location.href = response.redirectTo
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
        window.location.href = redirectPath
        return
      }

      await router.navigate({
        to: `/${userRealmId}${redirectPath}`,
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
      window.location.href = redirectTo
      return
    }

    // Prevent open redirect attacks
    let safeRedirectPath = getSafeRedirect(search.redirect, redirectPath)

    if (safeRedirectPath === '/') {
      safeRedirectPath = checkAdminPermission() ? '/manage' : '/user/profile'
    }

    if (safeRedirectPath.startsWith('http://') || safeRedirectPath.startsWith('https://')) {
      window.location.href = safeRedirectPath
      return
    }

    await router.navigate({
      to: `/${realmId}${safeRedirectPath}`,
      params: { realmId },
    })
  }

  if (consentStep) {
    return (
      <AuthPageWrapper>
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
      <AuthPageWrapper>
        <TotpVerificationForm
          realmId={realmId}
          tempToken={totpStep.tempToken}
          onSuccess={handleTotpSuccess}
          onBack={() => setTotpStep(null)}
        />
      </AuthPageWrapper>
    )
  }

  return (
    <AuthPageWrapper>
      <Card className="w-full max-w-md" data-testid="login-card">
        <CardHeader className="text-center">
          <CardTitle data-testid="login-title" className="text-2xl">
            {publicConfig?.realmName ?? 'Herald'}
          </CardTitle>
          <p className="text-sm text-muted-foreground">
            {publicConfig?.realmDescription || m['auth.login.login_to_account']()}
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
                      to="/$realmId/auth/forgot-password"
                      params={{ realmId }}
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
                    onClick={() => initiateOAuthLogin(realmId, provider.name)}
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
                to="/$realmId/auth/register"
                params={{ realmId }}
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
