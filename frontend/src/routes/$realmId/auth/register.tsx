import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { RegisterForm } from '@/components/auth/register-form'
import { publicConfigQueryOptions, queryKeys } from '@/data/query-options'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { useEffect } from 'react'
import { m } from '@/paraglide/messages'
import { realmPath, resolvedRealmFromPath } from '@/lib/realm-routing'

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

export function RegisterPage() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const pathname = window.location.pathname
  const realmContext = resolvedRealmFromPath(pathname)
  const { realmId } = realmContext

  const { data: publicConfig, isLoading, error } = useQuery(publicConfigQueryOptions(realmId))
  // Per-realm white-label config (FE-D02/FE-D03). Derived once so every register
  // sub-state (loading, error, disabled, form) reuses the same brand presentation.
  const whiteLabel = publicConfig?.whiteLabel ?? null
  const state = getRegisterPageState(publicConfig, isLoading, error)

  // Force refetch on mount to ensure fresh data
  useEffect(() => {
    queryClient.invalidateQueries({ queryKey: queryKeys.publicConfig(realmId) })
  }, [realmId, queryClient])

  function handleRegisterSuccess(verificationRequired: boolean): void {
    const destination = verificationRequired ? 'auth/verify-email' : 'auth/login'
    navigate({ to: realmPath(realmContext, destination) })
  }

  if (state.isLoading) {
    return (
      <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
        <div className="text-muted-foreground">{m['common.loading']()}</div>
      </AuthPageWrapper>
    )
  }

  if (state.error) {
    return (
      <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
        <div className="text-destructive">{m['auth.register.error_loading']()}</div>
      </AuthPageWrapper>
    )
  }

  if (!state.registrationAllowed) {
    return (
      <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
        <div className="w-full pt-8">
          <h1
            data-testid="registration-disabled-title"
            className="text-xl font-semibold tracking-tight"
          >
            {m['auth.register.disabled_title']()}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {m['auth.register.disabled_description']()}
          </p>
          <div className="mt-6">
            <Link
              to={realmPath(realmContext, '/auth/login')}
              className="text-sm font-medium text-primary hover:text-primary/80"
            >
              {m['auth.register.return_to_login']()}
            </Link>
          </div>
        </div>
      </AuthPageWrapper>
    )
  }

  return (
    <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
      <div className="w-full pt-8" data-testid="register-card">
        <h1 data-testid="register-title" className="text-xl font-semibold tracking-tight">
          {whiteLabel?.registerTitle ?? m['auth.register.title']()}
        </h1>
        {whiteLabel?.registerSubtitle ? (
          <p className="mt-1 text-sm text-muted-foreground" data-testid="register-subtitle">
            {whiteLabel.registerSubtitle}
          </p>
        ) : null}
        <div className="mt-6">
          <RegisterForm realmId={realmId} onSuccess={handleRegisterSuccess} />
          <div className="mt-4">
            <span className="text-sm text-muted-foreground">
              {m['auth.register.already_have_account']()}{' '}
            </span>
            <Link
              to={realmPath(realmContext, '/auth/login')}
              className="text-sm font-medium text-primary hover:text-primary/80"
              data-testid="login-link"
            >
              {m['auth.register.login_link']()}
            </Link>
          </div>
        </div>
      </div>
    </AuthPageWrapper>
  )
}
