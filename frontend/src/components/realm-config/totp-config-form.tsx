import React from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  totpConfigSchema,
  type TOTPConfigForm as TOTPConfigFormValues,
} from '@/lib/schemas/realm-config'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfigSwitchField } from './config-switch-field'
import { m } from '@/paraglide/messages'

interface TOTPConfigFormProps {
  realmId: string // For future use
  initialConfig?: TOTPConfigFormValues
  onSave: (config: TOTPConfigFormValues) => Promise<void>
  isLoading?: boolean
  disabled?: boolean
}

export function TOTPConfigForm({
  realmId: _realmId, // Renamed for clarity, unused in current implementation
  initialConfig,
  onSave,
  isLoading,
  disabled,
}: TOTPConfigFormProps) {
  const [isSubmitting, setIsSubmitting] = React.useState(false)

  const form = useAppForm({
    schema: totpConfigSchema,
    defaultValues: initialConfig || {
      enabled: false,
      forceEnabled: false,
    },
    onSubmit: async ({ value }) => {
      // Check if form is disabled
      if (disabled) {
        throw new Error(
          'Form is disabled. You do not have permission to modify this configuration.'
        )
      }

      // Prevent duplicate submissions
      if (isSubmitting) {
        console.log('Form is already submitting, ignoring duplicate submission')
        return
      }

      setIsSubmitting(true)
      try {
        await onSave(value)
      } catch (error) {
        // Log error for visibility but don't re-throw
        // The parent component should handle display of error messages
        console.error('Failed to save configuration:', error)
      } finally {
        setIsSubmitting(false)
      }
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>{m['realm_config.totp_title']()}</CardTitle>
        <CardDescription>{m['realm_config.totp_description']()}</CardDescription>
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
            {/* Enable TOTP */}
            <form.Field
              name="enabled"
              children={(field) => (
                <ConfigSwitchField
                  field={field}
                  form={form}
                  id="totp-enabled"
                  label={m['realm_config.totp_enable_label']()}
                  description={m['realm_config.totp_enable_description']()}
                  disabled={disabled}
                  errorTestId="totp-enabled-error"
                />
              )}
            />

            {/* Force TOTP */}
            <form.Field
              name="forceEnabled"
              children={(field) => (
                <ConfigSwitchField
                  field={field}
                  form={form}
                  id="totp-force-enabled"
                  label={m['realm_config.totp_force_label']()}
                  description={m['realm_config.totp_force_description']()}
                  disabled={disabled}
                  errorTestId="totp-force-enabled-error"
                />
              )}
            />

            <div className="flex justify-end">
              <Button
                type="submit"
                disabled={isLoading || isSubmitting || disabled}
                data-testid="totp-save-button"
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
