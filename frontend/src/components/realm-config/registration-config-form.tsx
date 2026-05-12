import React from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  registrationConfigSchema,
  type RegistrationConfigForm as RegistrationConfigFormValues,
} from '@/lib/schemas/realm-config'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfigSwitchField } from './config-switch-field'

interface RegistrationConfigFormProps {
  realmId: string // For future use
  initialConfig?: RegistrationConfigFormValues
  onSave: (config: RegistrationConfigFormValues) => Promise<void>
  isLoading?: boolean
  disabled?: boolean
}

export function RegistrationConfigForm({
  realmId: _realmId, // Renamed for clarity, unused in current implementation
  initialConfig,
  onSave,
  isLoading,
  disabled,
}: RegistrationConfigFormProps) {
  const [isSubmitting, setIsSubmitting] = React.useState(false)

  const form = useAppForm({
    schema: registrationConfigSchema,
    defaultValues: initialConfig || {
      allowed: true,
      requireEmailVerification: true,
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
        <CardTitle>Registration Configuration</CardTitle>
        <CardDescription>Configure user registration settings for this realm</CardDescription>
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
            {/* Allow Registration */}
            <form.Field
              name="allowed"
              children={(field) => (
                <ConfigSwitchField
                  field={field}
                  form={form}
                  id="reg-allowed"
                  label="Allow Registration"
                  description="Allow new users to register for this realm"
                  disabled={disabled}
                  errorTestId="reg-allowed-error"
                />
              )}
            />

            {/* Require Email Verification */}
            <form.Field
              name="requireEmailVerification"
              children={(field) => (
                <ConfigSwitchField
                  field={field}
                  form={form}
                  id="reg-require-email"
                  label="Require Email Verification"
                  description="Require users to verify their email address"
                  disabled={disabled}
                  errorTestId="reg-require-email-error"
                />
              )}
            />

            <div className="flex justify-end">
              <Button
                type="submit"
                disabled={isLoading || isSubmitting || disabled}
                data-testid="reg-save-button"
              >
                {isSubmitting ? 'Saving...' : 'Save'}
              </Button>
            </div>
          </form>
        </AppForm>
      </CardContent>
    </Card>
  )
}
