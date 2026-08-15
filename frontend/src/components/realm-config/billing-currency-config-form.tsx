import React from 'react'
import { useStore } from '@tanstack/react-form'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  billingCurrencyConfigSchema,
  type BillingCurrencyConfigForm as BillingCurrencyConfigFormValues,
} from '@/lib/schemas/realm-config'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { TextField } from '@/components/shared/form-fields/text-field'
import { m } from '@/paraglide/messages'
import { isValidCurrencyCode, normalizeCurrencyCode } from '@/lib/currency-utils'

interface BillingCurrencyConfigFormProps {
  initialConfig?: BillingCurrencyConfigFormValues
  onSave: (config: BillingCurrencyConfigFormValues) => Promise<void>
  isLoading?: boolean
  disabled?: boolean
}

/**
 * Realm default currency form. The backend rejects empty or invalid codes on
 * the generic realm-config upsert, so the currency can only be set or updated
 * here (no clear action); the effective default is simply left as-is when the
 * form is never saved.
 */
export function BillingCurrencyConfigForm({
  initialConfig,
  onSave,
  isLoading,
  disabled,
}: BillingCurrencyConfigFormProps) {
  const [isSubmitting, setIsSubmitting] = React.useState(false)

  const form = useAppForm({
    schema: billingCurrencyConfigSchema,
    defaultValues: initialConfig || {
      defaultCurrency: '',
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

  const defaultCurrency = useStore(form.store, (state) => state.values.defaultCurrency)
  const normalized = normalizeCurrencyCode(defaultCurrency)
  // Localized invalid hint: the schema's refine blocks submission, but its
  // message is not localized, so the visible copy comes from Paraglide here.
  const isInvalid = normalized !== '' && !isValidCurrencyCode(normalized)

  return (
    <Card>
      <CardHeader>
        <CardTitle>{m['realm_config.billing_currency_title']()}</CardTitle>
        <CardDescription>{m['realm_config.billing_currency_description']()}</CardDescription>
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
              name="defaultCurrency"
              label={m['realm_config.billing_currency_label']()}
              inputId="billing-default-currency"
              dataTestId="billing-default-currency-input"
              placeholder="USD"
              disabled={disabled}
            />

            {initialConfig?.defaultCurrency === '' && (
              <p className="text-sm text-muted-foreground">
                {m['realm_config.billing_currency_not_set']()}
              </p>
            )}
            {isInvalid && (
              <p className="text-sm text-destructive" data-testid="billing-currency-invalid-hint">
                {m['realm_config.billing_currency_invalid']()}
              </p>
            )}

            <div className="flex justify-end">
              <Button
                type="submit"
                disabled={isLoading || isSubmitting || disabled || normalized === '' || isInvalid}
                data-testid="billing-currency-save-button"
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
