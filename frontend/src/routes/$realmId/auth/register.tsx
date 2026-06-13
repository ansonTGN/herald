import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { RegisterForm } from '@/components/auth/register-form'
import { publicConfigQueryOptions, queryKeys } from '@/data/query-options'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Link } from '@tanstack/react-router'
import { useEffect } from 'react'
import { m } from '@/paraglide/messages'

export const Route = createFileRoute('/$realmId/auth/register')({
  component: RegisterPage,
})

interface RegisterPageState {
  isLoading: boolean
  error: boolean
  registrationAllowed: boolean
}

function getRegisterPageState(
  publicConfig: unknown | undefined,
  isLoading: boolean,
  error: unknown
): RegisterPageState {
  if (isLoading) {
    return { isLoading: true, error: false, registrationAllowed: false }
  }

  if (error) {
    return { isLoading: false, error: true, registrationAllowed: false }
  }

  const registrationEnabled =
    (publicConfig as { registration?: { enabled?: boolean } })?.registration?.enabled === true
  return { isLoading: false, error: false, registrationAllowed: registrationEnabled }
}

function RegisterPage() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const { data: publicConfig, isLoading, error } = useQuery(publicConfigQueryOptions(realmId))
  const state = getRegisterPageState(publicConfig, isLoading, error)

  // Force refetch on mount to ensure fresh data
  useEffect(() => {
    queryClient.invalidateQueries({ queryKey: queryKeys.publicConfig(realmId) })
  }, [realmId, queryClient])

  // Debug: Log config state
  console.log('[RegisterPage] publicConfig:', publicConfig)
  console.log('[RegisterPage] state:', state)

  function handleRegisterSuccess(verificationRequired: boolean): void {
    const destination = verificationRequired ? 'auth/verify-email' : 'auth/login'
    navigate({ to: `/${realmId}/${destination}` })
  }

  if (state.isLoading) {
    return (
      <AuthPageWrapper>
        <div className="text-gray-600">{m['common.loading']()}</div>
      </AuthPageWrapper>
    )
  }

  if (state.error) {
    return (
      <AuthPageWrapper>
        <div className="text-red-600">{m['auth.register.error_loading']()}</div>
      </AuthPageWrapper>
    )
  }

  if (!state.registrationAllowed) {
    return (
      <AuthPageWrapper>
        <Card className="max-w-md w-full">
          <CardHeader>
            <CardTitle data-testid="registration-disabled-title">
              {m['auth.register.disabled_title']()}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-gray-600 mb-4">{m['auth.register.disabled_description']()}</p>
            <Link
              to="/$realmId/auth/login"
              params={{ realmId }}
              className="text-primary hover:text-primary/80"
            >
              {m['auth.register.return_to_login']()}
            </Link>
          </CardContent>
        </Card>
      </AuthPageWrapper>
    )
  }

  return (
    <AuthPageWrapper>
      <Card className="max-w-md w-full" data-testid="register-card">
        <CardHeader>
          <CardTitle data-testid="register-title">{m['auth.register.title']()}</CardTitle>
        </CardHeader>
        <CardContent>
          <RegisterForm realmId={realmId} onSuccess={handleRegisterSuccess} />
          <div className="mt-4 text-center">
            <span className="text-sm text-gray-500">
              {m['auth.register.already_have_account']()}{' '}
            </span>
            <Link
              to="/$realmId/auth/login"
              params={{ realmId }}
              className="text-sm font-medium text-primary hover:text-primary/80"
              data-testid="login-link"
            >
              {m['auth.register.login_link']()}
            </Link>
          </div>
        </CardContent>
      </Card>
    </AuthPageWrapper>
  )
}
