import React from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  emailOtpConfigSchema,
  type EmailOtpConfigForm as EmailOtpConfigFormValues,
} from '@/lib/schemas/realm-config'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfigSwitchField } from './config-switch-field'
import { m } from '@/paraglide/messages'

interface EmailOtpConfigFormProps {
  initialConfig?: EmailOtpConfigFormValues
  onSave: (config: EmailOtpConfigFormValues) => Promise<void>
  isLoading?: boolean
  disabled?: boolean
}

export function EmailOtpConfigForm({
  initialConfig,
  onSave,
  isLoading,
  disabled,
}: EmailOtpConfigFormProps) {
  const [isSubmitting, setIsSubmitting] = React.useState(false)

  const form = useAppForm({
    schema: emailOtpConfigSchema,
    defaultValues: initialConfig || {
      enabled: false,
      autoRegister: false,
    },
    onSubmit: async ({ value }) => {
      setIsSubmitting(true)
      try {
        await onSave(value)
      } catch (error) {
        console.error('Failed to save configuration:', error)
      } finally {
        setIsSubmitting(false)
      }
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{m['realm_config.email_otp_title']()}</CardTitle>
        <CardDescription>{m['realm_config.email_otp_description']()}</CardDescription>
      </CardHeader>
      <CardContent>
        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              form.handleSubmit()
            }}
            className="space-y-4"
          >
            {/* Enable Email-OTP login */}
            <form.Field
              name="enabled"
              children={(field) => (
                <ConfigSwitchField
                  field={field}
                  form={form}
                  id="email-otp-enabled"
                  label={m['realm_config.email_otp_enable_label']()}
                  description={m['realm_config.email_otp_enable_description']()}
                  disabled={disabled}
                  errorTestId="email-otp-enabled-error"
                />
              )}
            />

            {/* Auto-register unverified emails on successful verification */}
            <form.Field
              name="autoRegister"
              children={(field) => (
                <ConfigSwitchField
                  field={field}
                  form={form}
                  id="email-otp-auto-register"
                  label={m['realm_config.email_otp_auto_register_label']()}
                  description={m['realm_config.email_otp_auto_register_description']()}
                  disabled={disabled}
                  errorTestId="email-otp-auto-register-error"
                />
              )}
            />

            <div className="flex justify-end">
              <Button
                type="submit"
                disabled={isLoading || isSubmitting || disabled}
                data-testid="email-otp-save-button"
              >
                {isSubmitting ? m['realm_config.saving']() : m['realm_config.save']()}
              </Button>
            </div>
          </form>
        </AppForm>
      </CardContent>
    </Card>
  )
}
