import { useState, useCallback } from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { QRCodeCanvas } from 'qrcode.react'
import { handleEnableTotp, handleVerifyTotpSetup } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Checkbox } from '@/components/ui/checkbox'
import { BackupCodesDisplay } from './backup-codes-display'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { withTimeout, type TotpData } from '@/lib/totp-utils'
import { z } from 'zod'
import type { EnableTotpResponse, VerifyTotpSetupResponse } from '@/lib/api-generated'

interface TotpSetupFormProps {
  realmId: string
  onSuccess: () => void
  onCancel: () => void
}

const totpSetupStep1Schema = z.object({
  password: z.string().min(1, 'Password is required'),
})

const totpSetupStep2Schema = z.object({
  code: z.string().length(6, 'Code must be 6 digits'),
})

type SetupStep = 'confirm' | 'verify'

function getSubmitButtonText(isSubmitting: boolean, action: 'generate' | 'verify'): string {
  const actionText = action === 'generate' ? 'Generating' : 'Verifying'
  const suffix = action === 'generate' ? ' QR Code' : ' & Enable TOTP'
  return isSubmitting
    ? `${actionText}...`
    : `${action.charAt(0).toUpperCase() + action.slice(1)}${suffix}`
}

export function TotpSetupForm({ onSuccess, onCancel }: TotpSetupFormProps) {
  const [step, setStep] = useState<SetupStep>('confirm')
  const [setupData, setSetupData] = useState<TotpData | null>(null)
  const [savedBackupCodes, setSavedBackupCodes] = useState(false)

  const generateMutation = useFormMutation({
    mutationFn: async (data: { password: string }) => {
      const response = await withTimeout(handleEnableTotp({ body: data }))
      if (response.error) {
        throw response.error
      }
      return response.data as EnableTotpResponse
    },
    getSuccessMessage: () => 'TOTP secret generated successfully',
    onSuccess: (data) => {
      setSetupData({
        secret: data.secret,
        qrCodeUrl: data.qrCodeUrl,
        backupCodes: data.backupCodes,
        tempToken: data.tempToken,
      })
      setStep('verify')
    },
  })

  const confirmForm = useAppForm({
    schema: totpSetupStep1Schema,
    defaultValues: { password: '' },
    onSubmit: async ({ value }) => {
      void generateMutation.mutate(value)
    },
  })

  const verifyMutation = useFormMutation({
    mutationFn: async (data: { code: string; tempToken: string }) => {
      const response = await withTimeout(handleVerifyTotpSetup({ body: data }))
      if (response.error) {
        throw response.error
      }
      return response.data as VerifyTotpSetupResponse
    },
    getSuccessMessage: () => 'TOTP enabled successfully',
    onSuccess: () => {
      onSuccess()
    },
  })

  const verifyForm = useAppForm({
    schema: totpSetupStep2Schema,
    defaultValues: { code: '' },
    onSubmit: async ({ value }) => {
      if (!savedBackupCodes || !setupData) return
      void verifyMutation.mutate({
        code: value.code,
        tempToken: setupData.tempToken,
      })
    },
  })

  const handleCheckboxChange = useCallback((checked: boolean | string) => {
    setSavedBackupCodes(checked === true)
  }, [])

  if (step === 'confirm') {
    return (
      <div className="space-y-4" data-testid="totp-setup-form">
        <h2 className="text-2xl font-bold">Enable TOTP</h2>
        <p className="text-muted-foreground">
          Enter your password to generate TOTP secret and backup recovery codes.
        </p>

        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              e.stopPropagation()
              confirmForm.handleSubmit()
            }}
            className="space-y-4"
          >
            <confirmForm.Field name="password">
              {(field) => (
                <div className="space-y-2">
                  <Label htmlFor="password">Current Password</Label>
                  <Input
                    id="password"
                    type="password"
                    value={field.state.value ?? ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    data-testid="totp-setup-password-input"
                  />
                  {(field.state.meta.isTouched || confirmForm.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-red-500" data-testid="password-error">
                        {getFieldErrorMessage(field.state.meta)}
                      </p>
                    )}
                </div>
              )}
            </confirmForm.Field>

            <div className="flex space-x-2">
              <Button
                type="button"
                variant="outline"
                onClick={onCancel}
                data-testid="totp-setup-cancel-button"
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={generateMutation.isSubmitting}
                data-testid="totp-setup-generate-button"
              >
                {getSubmitButtonText(generateMutation.isSubmitting, 'generate')}
              </Button>
            </div>
          </form>
        </AppForm>
      </div>
    )
  }

  return (
    <div className="space-y-4" data-testid="totp-setup-form-verify">
      <h2 className="text-2xl font-bold">Verify TOTP</h2>
      <p className="text-muted-foreground">
        Scan the QR code with your authenticator app, then enter the verification code.
      </p>

      {setupData && (
        <div className="flex justify-center" data-testid="totp-qr-code-container">
          <QRCodeCanvas value={setupData.qrCodeUrl} size={200} data-testid="totp-qr-code" />
          <input type="hidden" value={setupData.qrCodeUrl} data-testid="totp-qr-code-value" />
        </div>
      )}

      {setupData && <BackupCodesDisplay backupCodes={setupData.backupCodes} />}

      <div className="flex items-center space-x-2">
        <Checkbox
          id="saved-backup-codes"
          checked={savedBackupCodes}
          onCheckedChange={handleCheckboxChange}
          data-testid="totp-saved-backup-codes-checkbox"
        />
        <Label htmlFor="saved-backup-codes" data-testid="totp-saved-backup-codes-label">
          I have saved my backup codes
        </Label>
      </div>

      <AppForm>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            e.stopPropagation()
            verifyForm.handleSubmit()
          }}
          className="space-y-4"
        >
          <verifyForm.Field name="code">
            {(field) => (
              <div className="space-y-2">
                <Label htmlFor="code">Verification Code</Label>
                <Input
                  id="code"
                  type="text"
                  inputMode="numeric"
                  pattern="[0-9]{6}"
                  maxLength={6}
                  value={field.state.value ?? ''}
                  onChange={(e) => field.handleChange(e.target.value)}
                  data-testid="totp-verify-code-input"
                  placeholder="000000"
                  autoFocus
                />
                {(field.state.meta.isTouched || verifyForm.state.isSubmitted) &&
                  field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-red-500">{getFieldErrorMessage(field.state.meta)}</p>
                  )}
              </div>
            )}
          </verifyForm.Field>

          <div className="flex space-x-2">
            <Button
              type="button"
              variant="outline"
              onClick={onCancel}
              data-testid="totp-verify-cancel-button"
            >
              Cancel
            </Button>
            <Button
              type="submit"
              disabled={
                !savedBackupCodes || verifyMutation.isSubmitting || !verifyForm.state.canSubmit
              }
              data-testid="totp-verify-submit-button"
            >
              {getSubmitButtonText(verifyMutation.isSubmitting, 'verify')}
            </Button>
          </div>
        </form>
      </AppForm>
    </div>
  )
}
