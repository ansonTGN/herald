import { useState, useCallback } from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { QRCodeCanvas } from 'qrcode.react'
import { handleEnableTotp, handleVerifyTotpSetup } from '@/lib/api-generated'
import { obtainReauthToken } from '@/lib/reauth-flow'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Checkbox } from '@/components/ui/checkbox'
import { BackupCodesDisplay } from './backup-codes-display'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { withTimeout, type TotpData } from '@/lib/totp-utils'
import { z } from 'zod'
import { m } from '@/paraglide/messages'
import type { EnableTotpResponse, VerifyTotpSetupResponse } from '@/lib/api-generated'

interface TotpSetupFormProps {
  realmId: string
  onSuccess: () => void
  onCancel: () => void
}

const totpSetupStep1Schema = z.object({
  password: z.string().min(1, m['profile.totp_password_required']()),
})

const totpSetupStep2Schema = z.object({
  code: z.string().length(6, m['profile.totp_code_must_be_6_digits']()),
})

type SetupStep = 'confirm' | 'verify'

function getSubmitButtonText(isSubmitting: boolean, action: 'generate' | 'verify'): string {
  if (action === 'generate') {
    return isSubmitting ? m['profile.totp_generating']() : m['profile.totp_generate_button']()
  }
  return isSubmitting ? m['profile.totp_verifying']() : m['profile.totp_verify_enable_button']()
}

export function TotpSetupForm({ onSuccess, onCancel }: TotpSetupFormProps) {
  const [step, setStep] = useState<SetupStep>('confirm')
  const [setupData, setSetupData] = useState<TotpData | null>(null)
  const [savedBackupCodes, setSavedBackupCodes] = useState(false)

  const generateMutation = useFormMutation({
    mutationFn: async (data: { password: string }) => {
      // Bind-authenticator reauth: obtain a single-use ticket with the user's
      // password, then enable TOTP with it.
      const reauth_token = await obtainReauthToken('bind_authenticator', data.password)
      const response = await withTimeout(handleEnableTotp({ body: { ...data, reauth_token } }))
      if (response.error) {
        throw response.error
      }
      return response.data as EnableTotpResponse
    },
    getSuccessMessage: () => m['profile.totp_generated_success'](),
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
    getSuccessMessage: () => m['profile.totp_enabled_success'](),
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
        <h2 className="text-2xl font-bold">{m['profile.totp_setup_title']()}</h2>
        <p className="text-muted-foreground">{m['profile.totp_setup_description']()}</p>

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
                  <Label htmlFor="password">{m['profile.current_password_label']()}</Label>
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
                {m['common.cancel']()}
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
      <h2 className="text-2xl font-bold">{m['profile.totp_verify_title']()}</h2>
      <p className="text-muted-foreground">{m['profile.totp_verify_description']()}</p>

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
          {m['profile.totp_saved_backup_codes']()}
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
                <Label htmlFor="code">{m['profile.verification_code_label']()}</Label>
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
              {m['common.cancel']()}
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
