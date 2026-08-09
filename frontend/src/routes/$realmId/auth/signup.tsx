import { createFileRoute, useNavigate, Link } from '@tanstack/react-router'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { SignupForm } from '@/components/auth/signup-form'
import { publicConfigQueryOptions, signupStatusQueryOptions } from '@/data/query-options'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { m } from '@/paraglide/messages'
import { ADMIN_REALM_ID } from '@/lib/constants/auth-constants'
import { realmPath, resolvedRealmFromPath } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/auth/signup')({
  component: SignupPage,
})

// Self-service realm signup is an admin-realm-only public entry (DEC-001): the
// backend rejects any `realmId !== "admin"`. Branding still reads the admin
// realm's public config so the page presents the platform brand regardless of
// the URL it was opened under (main domain `/admin/auth/signup`, mirror
// `/auth/signup`, or a custom domain).
export function SignupPage() {
  const navigate = useNavigate()
  // Reused by the mirror `/auth/signup` route (same component), so resolve the
  // realm from the URL for link/navigation context — but the signup API and
  // branding are always the admin realm.
  const pathname = window.location.pathname
  const realmContext = resolvedRealmFromPath(pathname)

  const { data: publicConfig, isLoading: publicConfigLoading } = useQuery(
    publicConfigQueryOptions(ADMIN_REALM_ID)
  )
  const {
    data: signupStatus,
    isLoading: statusLoading,
    error: statusError,
  } = useQuery(signupStatusQueryOptions(ADMIN_REALM_ID))
  const whiteLabel = publicConfig?.whiteLabel ?? null

  const isLoading = publicConfigLoading || statusLoading
  const signupEnabled = signupStatus?.enabled === true

  function handleSignupSuccess(redirectPath: string, realmId: string): void {
    // Navigate into the NEW realm's management console (DEC-012).
    navigate({ to: realmPath({ realmId, isCustomDomain: false }, redirectPath) })
  }

  if (isLoading) {
    return (
      <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
        <div className="text-gray-600">{m['common.loading']()}</div>
      </AuthPageWrapper>
    )
  }

  // Fail-closed: a failed/absent status query means signup is treated as
  // disabled (DEC-013), matching the backend's missing-config → false behavior.
  if (statusError || !signupEnabled) {
    return (
      <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
        <Card className="max-w-md w-full">
          <CardHeader>
            <CardTitle data-testid="signup-disabled-notice">
              {m['auth.signup.disabled_title']()}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-gray-600 mb-4">{m['auth.signup.disabled_description']()}</p>
            <Link
              to={realmPath(realmContext, '/auth/login')}
              className="text-primary hover:text-primary/80"
            >
              {m['auth.signup.return_to_login']()}
            </Link>
          </CardContent>
        </Card>
      </AuthPageWrapper>
    )
  }

  return (
    <AuthPageWrapper whiteLabel={whiteLabel} realmName={publicConfig?.realmName}>
      <Card className="max-w-md w-full" data-testid="signup-card">
        <CardHeader>
          <CardTitle data-testid="signup-title">{m['auth.signup.title']()}</CardTitle>
          <p className="text-sm text-muted-foreground" data-testid="signup-subtitle">
            {m['auth.signup.subtitle']()}
          </p>
        </CardHeader>
        <CardContent>
          <SignupForm onSuccess={handleSignupSuccess} />
          <div className="mt-4 text-center">
            <span className="text-sm text-gray-500">
              {m['auth.signup.already_have_account']()}{' '}
            </span>
            <Link
              to={realmPath(realmContext, '/auth/login')}
              className="text-sm font-medium text-primary hover:text-primary/80"
              data-testid="login-link"
            >
              {m['auth.signup.login_link']()}
            </Link>
          </div>
        </CardContent>
      </Card>
    </AuthPageWrapper>
  )
}
