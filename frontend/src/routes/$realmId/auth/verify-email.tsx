import { useState, useEffect } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { verifyEmailConfirm, verifyEmailTrigger } from '@/lib/api-generated'
import { getErrorMessage } from '@/lib/error-utils'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { toast } from 'sonner'

const VERIFICATION_CODE_LENGTH = 6
const RESEND_COUNTDOWN_SECONDS = 60

function getResendButtonText(isResending: boolean, canResend: boolean, countdown: number): string {
  if (isResending) return 'Sending...'
  if (canResend) return 'Resend Verification Email'
  return `Resend in ${countdown}s`
}

export const Route = createFileRoute('/$realmId/auth/verify-email')({
  component: VerifyEmailPage,
})

function VerifyEmailPage() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()

  const [code, setCode] = useState('')
  const [email, setEmail] = useState('')
  const [countdown, setCountdown] = useState(RESEND_COUNTDOWN_SECONDS)
  const [canResend, setCanResend] = useState(false)
  const [verificationError, setVerificationError] = useState<string | null>(null)
  const [isVerifying, setIsVerifying] = useState(false)
  const [isResending, setIsResending] = useState(false)

  async function handleVerify(e: React.FormEvent) {
    e.preventDefault()
    if (code.length !== VERIFICATION_CODE_LENGTH) return

    setIsVerifying(true)
    setVerificationError(null)

    try {
      const response = await verifyEmailConfirm({
        path: { realmId, emailVerificationCode: code },
        throwOnError: true,
      })

      if (response.data) {
        toast.success('Email verified successfully')
        navigate({ to: `/${realmId}/auth/login` })
      }
    } catch (error) {
      setVerificationError(getErrorMessage(error))
    } finally {
      setIsVerifying(false)
    }
  }

  async function handleResend() {
    if (!canResend || isResending || !email) return

    setIsResending(true)
    setVerificationError(null)

    try {
      await verifyEmailTrigger({
        path: { realmId },
        body: { email, turnstileToken: undefined },
        throwOnError: true,
      })

      toast.success('Verification email sent successfully')
      setCountdown(RESEND_COUNTDOWN_SECONDS)
      setCanResend(false)
    } catch (error) {
      setVerificationError(getErrorMessage(error))
    } finally {
      setIsResending(false)
    }
  }

  useEffect(() => {
    if (countdown > 0) {
      const timer = setTimeout(() => setCountdown(countdown - 1), 1000)
      return () => clearTimeout(timer)
    } else {
      setCanResend(true)
    }
  }, [countdown])

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <Card className="max-w-md w-full">
        <CardHeader>
          <CardTitle data-testid="verify-email-title">Verify Your Email</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-gray-600 text-center mb-6">
            Please enter your email and 6-digit verification code sent to your email.
          </p>

          <form onSubmit={handleVerify} className="space-y-6">
            <div>
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                data-testid="verify-email-input"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="your@email.com"
                className="mt-1"
                required
              />
            </div>

            <div>
              <Label htmlFor="verification-code">Verification Code</Label>
              <Input
                id="verification-code"
                type="text"
                data-testid="verification-code-input"
                value={code}
                onChange={(e) =>
                  setCode(e.target.value.replace(/\D/g, '').slice(0, VERIFICATION_CODE_LENGTH))
                }
                placeholder="123456"
                maxLength={VERIFICATION_CODE_LENGTH}
                className="mt-1"
                required
              />
            </div>

            <Button
              type="submit"
              data-testid="verify-button"
              disabled={code.length !== VERIFICATION_CODE_LENGTH || isVerifying}
              className="w-full"
            >
              {isVerifying ? 'Verifying...' : 'Verify Email'}
            </Button>

            {verificationError && (
              <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg">
                {verificationError}
              </div>
            )}

            <div className="text-center">
              <Button
                type="button"
                variant="ghost"
                data-testid="resend-button"
                onClick={handleResend}
                disabled={!canResend || isResending}
              >
                {getResendButtonText(isResending, canResend, countdown)}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}
