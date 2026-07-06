import { useState } from 'react'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { z } from 'zod'
import { resetPasswordConfirm } from '@/lib/api-generated'
import { getErrorMessage } from '@/lib/error-utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AuthPageWrapper } from '@/components/auth/auth-page-wrapper'
import { TurnstileWidget } from '@/components/auth/turnstile-widget'
import { turnstileStatusQueryOptions } from '@/data/query-options'
import { toast } from 'sonner'
import { m } from '@/paraglide/messages'

const resetPasswordSearchSchema = z.object({
  code: z.string().min(1),
})

export const Route = createFileRoute('/$realmId/auth/reset-password')({
  component: ResetPasswordPage,
  validateSearch: (search) => resetPasswordSearchSchema.parse(search),
})

function ResetPasswordPage() {
  const { realmId } = Route.useParams()
  const { code } = Route.useSearch()
  const navigate = useNavigate()

  const [newPassword, setNewPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [turnstileToken, setTurnstileToken] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const { data: turnstileStatus, isLoading: loadingTurnstile } = useQuery(
    turnstileStatusQueryOptions(realmId)
  )

  const passwordsMatch = newPassword.length >= 8 && newPassword === confirmPassword

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault()
    if (isSubmitting || !passwordsMatch) return

    setIsSubmitting(true)
    setError(null)

    try {
      await resetPasswordConfirm({
        path: { realmId, resetCode: code },
        body: { newPass: newPassword, turnstileToken },
        throwOnError: true,
      })
      toast.success(m['auth.reset_password.success']())
      navigate({ to: `/${realmId}/auth/login` })
    } catch (err) {
      setError(getErrorMessage(err))
    } finally {
      setIsSubmitting(false)
    }
  }

  const showMismatchError = confirmPassword.length > 0 && newPassword !== confirmPassword

  return (
    <AuthPageWrapper>
      <Card className="w-full max-w-md" data-testid="reset-password-card">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl" data-testid="reset-password-title">
            {m['auth.reset_password.title']()}
          </CardTitle>
          <p className="text-sm text-muted-foreground">{m['auth.reset_password.description']()}</p>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4" data-testid="reset-password-form">
            {error && (
              <div
                className="p-3 bg-red-50 border border-red-200 rounded text-red-600 text-sm"
                data-testid="reset-password-error"
              >
                {error}
              </div>
            )}

            <div>
              <Label htmlFor="new-password">{m['auth.reset_password.new_password_label']()}</Label>
              <Input
                id="new-password"
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                disabled={isSubmitting}
                required
                minLength={8}
                autoFocus
                className="mt-1"
                data-testid="reset-password-new-input"
              />
              {newPassword.length > 0 && newPassword.length < 8 && (
                <p className="text-sm text-red-500 mt-1">{m['auth.password_min_length']()}</p>
              )}
            </div>

            <div>
              <Label htmlFor="confirm-password">
                {m['auth.reset_password.confirm_password_label']()}
              </Label>
              <Input
                id="confirm-password"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                disabled={isSubmitting}
                required
                minLength={8}
                className="mt-1"
                data-testid="reset-password-confirm-input"
              />
              {showMismatchError && (
                <p className="text-sm text-red-500 mt-1">{m['auth.passwords_dont_match']()}</p>
              )}
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
              disabled={isSubmitting || !passwordsMatch}
              className="w-full"
              data-testid="reset-password-submit-button"
            >
              {isSubmitting
                ? m['auth.reset_password.submitting']()
                : m['auth.reset_password.submit']()}
            </Button>
          </form>

          <div className="mt-4 text-center">
            <Link
              to="/$realmId/auth/login"
              params={{ realmId }}
              className="text-sm font-medium text-primary hover:text-primary/80"
              data-testid="reset-password-back-link"
            >
              {m['auth.reset_password.back_to_login']()}
            </Link>
          </div>
        </CardContent>
      </Card>
    </AuthPageWrapper>
  )
}
