import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { RegisterForm } from '@/components/auth/register-form'
import { publicConfigQueryOptions, queryKeys } from '@/data/query-options'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Link } from '@tanstack/react-router'
import { useEffect } from 'react'

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

  const allowed =
    (publicConfig as { registration?: { allowed?: boolean } })?.registration?.allowed === true
  return { isLoading: false, error: false, registrationAllowed: allowed }
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
        <div className="text-gray-600">Loading...</div>
      </AuthPageWrapper>
    )
  }

  if (state.error) {
    return (
      <AuthPageWrapper>
        <div className="text-red-600">Error loading registration configuration</div>
      </AuthPageWrapper>
    )
  }

  if (!state.registrationAllowed) {
    return (
      <AuthPageWrapper>
        <Card className="max-w-md w-full">
          <CardHeader>
            <CardTitle data-testid="registration-disabled-title">
              Registration Not Available
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-gray-600 mb-4">
              Registration is not enabled for this realm. Please contact an administrator.
            </p>
            <Link
              to="/$realmId/auth/login"
              params={{ realmId }}
              className="text-blue-600 hover:text-blue-700"
            >
              Return to Login
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
          <CardTitle data-testid="register-title">Create an Account</CardTitle>
        </CardHeader>
        <CardContent>
          <RegisterForm realmId={realmId} onSuccess={handleRegisterSuccess} />
          <div className="mt-4 text-center">
            <span className="text-sm text-gray-500">Already have an account? </span>
            <Link
              to="/$realmId/auth/login"
              params={{ realmId }}
              className="text-sm font-medium text-blue-600 hover:text-blue-500"
              data-testid="login-link"
            >
              Login
            </Link>
          </div>
        </CardContent>
      </Card>
    </AuthPageWrapper>
  )
}
