import { useState } from 'react'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { resetPasswordRequest } from '@/lib/api-generated'
import { getErrorMessage } from '@/lib/error-utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { TurnstileWidget } from '@/components/auth/turnstile-widget'
import { publicConfigQueryOptions, turnstileStatusQueryOptions } from '@/data/query-options'
import { toast } from 'sonner'
import { m } from '@/paraglide/messages'

export const Route = createFileRoute('/$realmId/auth/forgot-password')({
  component: ForgotPasswordPage,
})

function ForgotPasswordPage() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()

  const [email, setEmail] = useState('')
  const [turnstileToken, setTurnstileToken] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [sent, setSent] = useState(false)

  const { data: turnstileStatus, isLoading: loadingTurnstile } = useQuery(
    turnstileStatusQueryOptions(realmId)
  )
  const { data: publicConfig } = useQuery(publicConfigQueryOptions(realmId))
  // Per-realm white-label config (FE-D03). Generic variant: logo/accent/
  // background/footer only, never login/register copy.
  const whiteLabel = publicConfig?.whiteLabel ?? null

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (isSubmitting || !email) return

    setIsSubmitting(true)
    setError(null)

    try {
      await resetPasswordRequest({
        path: { realmId },
        body: { email, turnstileToken },
        throwOnError: true,
      })
      setSent(true)
      toast.success(m['auth.forgot_password.success']())
    } catch (err) {
      setError(getErrorMessage(err))
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <AuthPageWrapper whiteLabel={whiteLabel}>
      <Card className="w-full max-w-md" data-testid="forgot-password-card">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl" data-testid="forgot-password-title">
            {m['auth.forgot_password.title']()}
          </CardTitle>
          <p className="text-sm text-muted-foreground">{m['auth.forgot_password.description']()}</p>
        </CardHeader>
        <CardContent>
          {sent ? (
            <div className="space-y-4" data-testid="forgot-password-success">
              <div className="p-3 bg-green-50 border border-green-200 rounded text-green-700 text-sm">
                {m['auth.forgot_password.success']()}
              </div>
              <Button
                type="button"
                variant="outline"
                className="w-full"
                onClick={() => navigate({ to: `/${realmId}/auth/login` })}
              >
                {m['auth.forgot_password.back_to_login']()}
              </Button>
            </div>
          ) : (
            <form onSubmit={handleSubmit} className="space-y-4" data-testid="forgot-password-form">
              {error && (
                <div
                  className="p-3 bg-red-50 border border-red-200 rounded text-red-600 text-sm"
                  data-testid="forgot-password-error"
                >
                  {error}
                </div>
              )}

              <div>
                <Label htmlFor="email">{m['auth.forgot_password.email_label']()}</Label>
                <Input
                  id="email"
                  type="email"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isSubmitting}
                  placeholder="your@email.com"
                  required
                  autoFocus
                  className="mt-1"
                  data-testid="forgot-password-email-input"
                />
              </div>

              {!loadingTurnstile && turnstileStatus?.enabled && (
                <TurnstileWidget
                  siteKey={turnstileStatus.site_key || ''}
                  onTokenChange={setTurnstileToken}
                  onError={(error) => console.error('Turnstile error:', error)}
                />
              )}

              <Button
                type="submit"
                disabled={isSubmitting || !email}
                className="w-full"
                data-testid="forgot-password-submit-button"
              >
                {isSubmitting
                  ? m['auth.forgot_password.submitting']()
                  : m['auth.forgot_password.submit']()}
              </Button>
            </form>
          )}

          <div className="mt-4 text-center">
            <Link
              to="/$realmId/auth/login"
              params={{ realmId }}
              className="text-sm font-medium text-primary hover:text-primary/80"
              data-testid="forgot-password-back-link"
            >
              {m['auth.forgot_password.back_to_login']()}
            </Link>
          </div>
        </CardContent>
      </Card>
    </AuthPageWrapper>
  )
}
