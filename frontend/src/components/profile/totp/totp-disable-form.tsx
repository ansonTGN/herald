import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { handleDisableTotp } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { withTimeout } from '@/lib/totp-utils'
import { z } from 'zod'
import { m } from '@/paraglide/messages'
import type { DisableTotpResponse } from '@/lib/api-generated'

const disableTotpSchema = z.object({
  password: z.string().min(1, m['profile.totp_password_required']()),
})

interface TotpDisableFormProps {
  onSuccess: () => void
  onCancel: () => void
  isForceTotpEnabled: boolean
}

function getSubmitButtonText(isSubmitting: boolean): string {
  return isSubmitting ? m['profile.totp_disabling']() : m['profile.totp_disable_button']()
}

export function TotpDisableForm({ onSuccess, onCancel, isForceTotpEnabled }: TotpDisableFormProps) {
  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: async (data: { password: string }) => {
      const response = await withTimeout(handleDisableTotp({ body: data }))
      return response.data as DisableTotpResponse
    },
    getSuccessMessage: () => m['profile.totp_disabled_success'](),
    onSuccess: () => {
      onSuccess()
    },
  })

  const form = useAppForm({
    schema: disableTotpSchema,
    defaultValues: { password: '' },
    onSubmit: async ({ value }) => {
      void mutate(value)
    },
  })

  if (isForceTotpEnabled) {
    return (
      <Alert data-testid="totp-force-enabled-alert">
        <AlertDescription data-testid="totp-force-enabled-message">
          {m['profile.totp_force_enabled_message']()}
        </AlertDescription>
      </Alert>
    )
  }

  return (
    <div className="space-y-4" data-testid="totp-disable-form">
      <h2 className="text-2xl font-bold">{m['profile.totp_disable_title']()}</h2>
      <p className="text-muted-foreground">{m['profile.totp_disable_description']()}</p>

      <AppForm>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            e.stopPropagation()
            form.handleSubmit()
          }}
          className="space-y-4"
        >
          <form.Field name="password">
            {(field) => (
              <div className="space-y-2">
                <Label htmlFor="password">{m['profile.current_password_label']()}</Label>
                <Input
                  id="password"
                  type="password"
                  value={field.state.value ?? ''}
                  onChange={(e) => field.handleChange(e.target.value)}
                  data-testid="totp-disable-password-input"
                />
                {(field.state.meta.isTouched || form.state.isSubmitted) &&
                  field.state.meta.errors.length > 0 && (
                    <p className="text-sm text-red-500">{getFieldErrorMessage(field.state.meta)}</p>
                  )}
              </div>
            )}
          </form.Field>

          <div className="flex space-x-2">
            <Button
              type="button"
              variant="outline"
              onClick={onCancel}
              data-testid="totp-disable-cancel-button"
            >
              {m['common.cancel']()}
            </Button>
            <Button
              type="submit"
              variant="destructive"
              disabled={isSubmitting}
              data-testid="totp-disable-submit-button"
            >
              {getSubmitButtonText(isSubmitting)}
            </Button>
          </div>
        </form>
      </AppForm>
    </div>
  )
}
