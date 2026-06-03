import React from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  turnstileConfigSchema,
  type TurnstileConfigForm as TurnstileConfigFormValues,
} from '@/lib/schemas/realm-config'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { TextField } from '@/components/shared/form-fields/text-field'

const MASKED_SECRET = '••••••••'

interface TurnstileConfigFormProps {
  realmId: string
  initialConfig?: TurnstileConfigFormValues
  onSave: (config: TurnstileConfigFormValues) => Promise<void>
  isLoading?: boolean
  disabled?: boolean
}

export function TurnstileConfigForm({
  realmId: _realmId,
  initialConfig,
  onSave,
  isLoading,
  disabled,
}: TurnstileConfigFormProps) {
  const [isSubmitting, setIsSubmitting] = React.useState(false)

  const form = useAppForm({
    schema: turnstileConfigSchema,
    defaultValues: initialConfig || {
      siteKey: '',
      secretKey: '',
    },
    onSubmit: async ({ value }) => {
      if (disabled) {
        throw new Error(
          'Form is disabled. You do not have permission to modify this configuration.'
        )
      }

      if (isSubmitting) {
        return
      }

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
        <CardTitle>Turnstile Configuration</CardTitle>
        <CardDescription>
          Configure Cloudflare Turnstile verification for this realm
        </CardDescription>
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
            <TextField
              form={form}
              name="siteKey"
              label="Site Key"
              inputId="turnstile-site-key"
              dataTestId="turnstile-site-key-input"
              placeholder="0x4AAAAAAA..."
              disabled={disabled}
            />

            <TextField
              form={form}
              name="secretKey"
              label="Secret Key"
              inputId="turnstile-secret-key"
              dataTestId="turnstile-secret-key-input"
              placeholder={MASKED_SECRET}
              disabled={disabled}
              type="password"
            />

            <div className="flex justify-end">
              <Button
                type="submit"
                disabled={isLoading || isSubmitting || disabled}
                data-testid="turnstile-save-button"
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
