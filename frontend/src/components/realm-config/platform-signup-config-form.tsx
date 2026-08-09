import React from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  platformSignupConfigSchema,
  type PlatformSignupConfigForm as PlatformSignupConfigFormValues,
} from '@/lib/schemas/realm-config'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfigSwitchField } from './config-switch-field'
import { m } from '@/paraglide/messages'

interface PlatformSignupConfigFormProps {
  initialConfig?: PlatformSignupConfigFormValues
  onSave: (config: PlatformSignupConfigFormValues) => Promise<void>
  isLoading?: boolean
  disabled?: boolean
}

// Admin-realm-only platform self-service signup toggle (DEC-009/013). The
// settings page mounts this only when realmId === 'admin' and the caller has
// SETTINGS_MANAGE; the form itself is a single-switch form mirroring the
// registration config form's structure.
export function PlatformSignupConfigForm({
  initialConfig,
  onSave,
  isLoading,
  disabled,
}: PlatformSignupConfigFormProps) {
  const [isSubmitting, setIsSubmitting] = React.useState(false)

  const form = useAppForm({
    schema: platformSignupConfigSchema,
    defaultValues: initialConfig ?? { enabled: false },
    onSubmit: async ({ value }) => {
      setIsSubmitting(true)
      try {
        await onSave(value)
      } catch (error) {
        // Parent component surfaces the error via toast; logged for visibility.
        console.error('Failed to save platform signup configuration:', error)
      } finally {
        setIsSubmitting(false)
      }
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{m['settings.platform_signup.title']()}</CardTitle>
        <CardDescription>{m['settings.platform_signup.description']()}</CardDescription>
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
            <form.Field
              name="enabled"
              children={(field) => (
                <ConfigSwitchField
                  field={field}
                  form={form}
                  id="platform-signup"
                  label={m['settings.platform_signup.enable_label']()}
                  description={m['settings.platform_signup.enable_description']()}
                  disabled={disabled}
                  errorTestId="platform-signup-error"
                />
              )}
            />

            <div className="flex justify-end">
              <Button
                type="submit"
                disabled={isLoading || isSubmitting || disabled}
                data-testid="platform-signup-save-button"
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
