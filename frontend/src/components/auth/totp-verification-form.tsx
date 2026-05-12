import { useState, useCallback } from 'react'
import { useMutation } from '@tanstack/react-query'
import { handleVerifyTotp } from '@/lib/api-generated'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Shield, RefreshCw } from 'lucide-react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { withTimeout } from '@/lib/totp-utils'
import { z } from 'zod'
import type { VerifyTotpResponse } from '@/lib/api-generated'

const totpCodeSchema = z.object({
  code: z.string().length(6, 'Code must be 6 digits'),
})

const backupCodeSchema = z.object({
  code: z.string().length(8, 'Code must be 8 characters'),
})

interface TotpVerificationFormProps {
  realmId: string
  tempToken: string
  onSuccess: (token: string) => void
  onBack?: () => void
}

type CodeType = 'totp' | 'backup'

const MAX_ATTEMPTS = 5
const TOTP_CODE_LENGTH = 6
const BACKUP_CODE_LENGTH = 8
const LOCKED_MESSAGE = 'Too many failed attempts. Please try again in 15 minutes.'

export function TotpVerificationForm({
  realmId,
  tempToken,
  onSuccess,
  onBack,
}: TotpVerificationFormProps) {
  const [codeType, setCodeType] = useState<CodeType>('totp')
  const [attempts, setAttempts] = useState(0)
  const [error, setError] = useState<string | null>(null)

  const totpForm = useAppForm({
    schema: totpCodeSchema,
    defaultValues: { code: '' },
    onSubmit: async ({ value }) => {
      if (attempts >= MAX_ATTEMPTS) return
      setError(null)
      verifyMutation.mutate({
        code: value.code,
        backupCode: null,
      })
    },
  })

  const backupForm = useAppForm({
    schema: backupCodeSchema,
    defaultValues: { code: '' },
    onSubmit: async ({ value }) => {
      if (attempts >= MAX_ATTEMPTS) return
      setError(null)
      verifyMutation.mutate({
        code: '',
        backupCode: value.code.toUpperCase(),
      })
    },
  })

  const verifyMutation = useMutation({
    mutationFn: async (data: { code: string; backupCode: string | null }) => {
      const response = await withTimeout(
        handleVerifyTotp({
          path: { realmId },
          body: {
            code: data.backupCode ? undefined : data.code,
            backupCode: data.backupCode,
            tempToken,
          },
        })
      )
      return response.data as VerifyTotpResponse
    },
    onSuccess: (data) => {
      onSuccess(data.token)
    },
    onError: (err: unknown) => {
      setAttempts((prev) => prev + 1)
      const errorMessage =
        err && typeof err === 'object' && 'error' in err
          ? String(err.error)
          : err && typeof err === 'object' && 'message' in err
            ? String(err.message)
            : 'Invalid code. Please try again.'
      setError(errorMessage)
    },
  })

  const currentForm = codeType === 'backup' ? backupForm : totpForm
  const codeLength = codeType === 'backup' ? BACKUP_CODE_LENGTH : TOTP_CODE_LENGTH
  const remainingAttempts = MAX_ATTEMPTS - attempts
  const isLocked = attempts >= MAX_ATTEMPTS

  const handleCodeChange = useCallback(
    (value: string) => {
      const processedValue = codeType === 'backup' ? value.toUpperCase() : value
      currentForm.setFieldValue('code', processedValue)

      if (processedValue.length === codeLength && !verifyMutation.isPending && !isLocked) {
        setError(null)
        currentForm.handleSubmit()
      }
    },
    [codeType, codeLength, currentForm, verifyMutation.isPending, isLocked]
  )

  const switchToBackupCode = useCallback(() => {
    setCodeType('backup')
    setError(null)
    backupForm.reset()
  }, [backupForm])

  const switchToTotpCode = useCallback(() => {
    setCodeType('totp')
    setError(null)
    totpForm.reset()
  }, [totpForm])

  const getInputType = useCallback((): 'text' | 'numeric' => {
    return codeType === 'backup' ? 'text' : 'numeric'
  }, [codeType])

  const getInputPattern = useCallback((): string => {
    return codeType === 'backup' ? '[A-Z0-9]{8}' : '[0-9]{6}'
  }, [codeType])

  const getPlaceholder = useCallback((): string => {
    return codeType === 'backup' ? 'XXXXXXXX' : '000000'
  }, [codeType])

  const getLabelText = useCallback((): string => {
    return codeType === 'backup' ? 'Backup Code' : 'Verification Code'
  }, [codeType])

  const getDescriptionText = useCallback((): string => {
    return codeType === 'backup'
      ? 'Enter one of your backup recovery codes'
      : 'Enter the 6-digit code from your authenticator app'
  }, [codeType])

  const getAttemptsText = useCallback((): string | null => {
    if (isLocked || remainingAttempts <= 0 || remainingAttempts >= MAX_ATTEMPTS) return null
    return `${remainingAttempts} attempt${remainingAttempts > 1 ? 's' : ''} remaining`
  }, [isLocked, remainingAttempts])

  return (
    <div className="flex min-h-screen items-center justify-center">
      <Card className="w-full max-w-md" data-testid="totp-verification-form">
        <CardHeader>
          <CardTitle className="flex items-center space-x-2">
            <Shield className="h-5 w-5" />
            <span>Two-Factor Authentication</span>
          </CardTitle>
          <CardDescription>{getDescriptionText()}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {error && (
            <div className="text-sm text-destructive" data-testid="totp-verification-error">
              {error}
            </div>
          )}

          {isLocked && (
            <div className="text-sm text-destructive" data-testid="totp-verification-locked">
              {LOCKED_MESSAGE}
            </div>
          )}

          <AppForm>
            <form
              onSubmit={(e) => {
                e.preventDefault()
                e.stopPropagation()
                currentForm.handleSubmit()
              }}
              className="space-y-4"
            >
              <currentForm.Field name="code">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="code">{getLabelText()}</Label>
                    <Input
                      id="code"
                      type={getInputType()}
                      inputMode={getInputType()}
                      pattern={getInputPattern()}
                      maxLength={codeLength}
                      value={field.state.value ?? ''}
                      onChange={(e) => handleCodeChange(e.target.value)}
                      disabled={isLocked || verifyMutation.isPending}
                      data-testid="totp-verification-code-input"
                      placeholder={getPlaceholder()}
                      autoFocus
                    />
                    {(field.state.meta.isTouched || currentForm.state.isSubmitted) &&
                      field.state.meta.errors.length > 0 && (
                        <p className="text-sm text-red-500">
                          {getFieldErrorMessage(field.state.meta)}
                        </p>
                      )}
                  </div>
                )}
              </currentForm.Field>
            </form>
          </AppForm>

          {codeType === 'totp' && !isLocked && (
            <button
              type="button"
              onClick={switchToBackupCode}
              className="text-sm text-primary hover:underline"
              data-testid="totp-use-backup-code-link"
            >
              Use a backup code instead
            </button>
          )}

          {codeType === 'backup' && !isLocked && (
            <button
              type="button"
              onClick={switchToTotpCode}
              className="text-sm text-primary hover:underline"
              data-testid="totp-use-totp-code-link"
            >
              Use TOTP code instead
            </button>
          )}

          {getAttemptsText() && (
            <div className="text-sm text-muted-foreground" data-testid="totp-remaining-attempts">
              {getAttemptsText()}
            </div>
          )}

          {onBack && (
            <Button
              type="button"
              variant="ghost"
              onClick={onBack}
              className="w-full"
              data-testid="totp-verification-back-button"
            >
              <RefreshCw className="mr-2 h-4 w-4" />
              Back to Login
            </Button>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
