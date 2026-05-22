import { createFileRoute, useRouter } from '@tanstack/react-router'
import { useForm } from '@tanstack/react-form'
import { useMutation, useQuery } from '@tanstack/react-query'
import type { LoginRequestPayload, VerifyTotpResponse } from '@/lib/api-generated'
import { loginSchema } from '@/lib/schemas/common'
import { loginSearchSchema, type LoginSearchParams } from '@/lib/schemas/search-params'
import { getErrorMessage, getFieldErrorMessage } from '@/lib/error-utils'
import {
  loginFlow,
  completeLoginAfterTotp,
  getSafeRedirect,
  checkAdminPermission,
  validateOAuthParams,
  type LoginFlowResult,
} from '@/lib/auth-utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { TotpVerificationForm } from '@/components/auth/totp-verification-form'
import { publicConfigQueryOptions } from '@/data/query-options'
import { Link } from '@tanstack/react-router'
import { useOAuthLogin } from '@/hooks/use-oauth-login'
import { useState } from 'react'
import { toast } from 'sonner'

interface TotpStep {
  tempToken: string
}

const DEFAULT_CLIENT_ID = 'admin-web-console'

export const Route = createFileRoute('/$realmId/auth/login')({
  component: LoginPage,
  validateSearch: (search) => loginSearchSchema.parse(search),
  // NOTE: Authentication and redirect logic is handled by __root.tsx
  // This allows us to use cached auth data and avoid redundant API calls
})

function LoginPage() {
  const router = useRouter()
  const { realmId } = Route.useParams()
  const search = Route.useSearch() as LoginSearchParams
  const { initiateOAuthLogin } = useOAuthLogin()

  const [totpStep, setTotpStep] = useState<TotpStep | null>(null)
  const [globalError, setGlobalError] = useState<string | null>(null)

  const { data: publicConfig, isLoading } = useQuery(publicConfigQueryOptions(realmId))

  const oauthProviders = publicConfig?.oauthProviders ?? []
  const isRegistrationEnabled = publicConfig?.registration?.allowed === true

  const { oauthParams, hasPartialOAuth } = validateOAuthParams(search)

  const loginMutation = useMutation({
    mutationFn: async (values: { username: string; password: string }) => {
      const isEmail = values.username.includes('@')
      const clientId = search.clientId || DEFAULT_CLIENT_ID

      const loginData: LoginRequestPayload = {
        clientId,
        email: isEmail ? values.username : undefined,
        username: isEmail ? undefined : values.username,
        password: values.password,
        ...(oauthParams ?? {}),
      }

      return await loginFlow(realmId, loginData)
    },
    onSuccess: async (data: LoginFlowResult) => {
      setGlobalError(null)

      if (data.response.requiresTotp && data.response.tempToken) {
        setTotpStep({ tempToken: data.response.tempToken })
      } else if (data.response.redirectTo) {
        window.location.href = data.response.redirectTo
        return
      } else {
        toast.success('Login successful')

        const userRealmId = data.response.realmId || realmId
        let redirectPath = search.redirect || data.redirectPath

        // Prevent open redirect attacks
        redirectPath = getSafeRedirect(redirectPath)

        if (redirectPath === '/') {
          redirectPath = checkAdminPermission() ? '/manage' : '/user/profile'
        }

        await router.navigate({
          to: `/${userRealmId}${redirectPath}`,
          params: { realmId: userRealmId },
        })
      }
    },
  })

  const form = useForm({
    defaultValues: { username: '', password: '' },
    onSubmit: async ({ value }) => {
      setGlobalError(null)
      if (hasPartialOAuth) return
      loginMutation.mutate(value, {
        onError: (error: unknown) => {
          const message = getErrorMessage(error)
          toast.error(message)
          setGlobalError(message)
        },
      })
    },
  })

  async function handleTotpSuccess(verifyResponse: VerifyTotpResponse): Promise<void> {
    toast.success('Login successful')

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

    await router.navigate({
      to: `/${realmId}${safeRedirectPath}`,
      params: { realmId },
    })
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
            {publicConfig?.realmDescription || 'Login to your account'}
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
              OAuth 参数不完整。请通过正确的 OAuth 链接访问此页面。
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
                  <Label htmlFor="username">Username or Email</Label>
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
                  <Label htmlFor="password">Password</Label>
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

            <Button
              type="submit"
              disabled={loginMutation.isPending || hasPartialOAuth}
              className="w-full"
              data-testid="login-submit-button"
            >
              {loginMutation.isPending ? 'Logging in...' : 'Login'}
            </Button>
          </form>

          {!isLoading && oauthProviders.length > 0 && (
            <div className="space-y-3 mt-6">
              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-background px-2 text-muted-foreground">Or continue with</span>
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

          {isRegistrationEnabled && (
            <div className="mt-4 text-center">
              <span className="text-sm text-gray-500">Don't have an account? </span>
              <Link
                to="/$realmId/auth/register"
                params={{ realmId }}
                className="text-sm font-medium text-blue-600 hover:text-blue-500"
                data-testid="register-link"
              >
                Register
              </Link>
            </div>
          )}
        </CardContent>
      </Card>
    </AuthPageWrapper>
  )
}
