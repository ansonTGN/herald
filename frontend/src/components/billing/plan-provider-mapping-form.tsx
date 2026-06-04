import { useEffect, useMemo } from 'react'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { TextField } from '@/components/shared/form-fields'
import {
  providerMappingSchema,
  type ProviderMappingFormData,
  getProviderMappingDefaults,
} from '@/lib/schemas/billing-forms'
import type { SubscriptionPlanPaymentProviderResponse } from '@/lib/api-generated'
import { formatProviderName } from './format-provider-name'
import { m } from '@/paraglide/messages'

interface PlanProviderMappingFormProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: ProviderMappingFormData) => void
  isSubmitting: boolean
  mapping?: SubscriptionPlanPaymentProviderResponse
  realmId: string
  availableProviders: string[]
}

export function PlanProviderMappingForm({
  open,
  onOpenChange,
  onSubmit,
  isSubmitting,
  mapping,
  availableProviders,
}: PlanProviderMappingFormProps) {
  const isEditing = !!mapping

  const defaultValues = useMemo(
    () =>
      getProviderMappingDefaults(
        mapping
          ? {
              paymentProvider: mapping.paymentProvider,
              externalProductId: mapping.externalProductId,
              externalPriceId: mapping.externalPriceId,
              enabled: mapping.enabled,
            }
          : undefined
      ),
    [mapping]
  )

  const form = useAppForm({
    schema: providerMappingSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      await onSubmit(value)
    },
  })

  useEffect(() => {
    if (open) {
      form.reset(defaultValues)
    }
  }, [defaultValues, form, open])

  return (
    <BaseFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={
        isEditing ? m['billing.edit_payment_provider']() : m['billing.add_payment_provider_title']()
      }
      description={
        isEditing ? m['billing.edit_mapping_description']() : m['billing.add_mapping_description']()
      }
      className="max-w-lg"
      isSubmitting={isSubmitting}
      data-testid="provider-mapping-form-dialog"
      footer={
        <>
          <Button
            type="button"
            variant="outline"
            data-testid="provider-mapping-cancel-button"
            onClick={() => onOpenChange(false)}
          >
            {m['common.cancel']()}
          </Button>
          <Button
            type="submit"
            form="provider-mapping-form"
            disabled={isSubmitting || (!isEditing && availableProviders.length === 0)}
            data-testid="provider-mapping-submit-button"
          >
            {isSubmitting
              ? m['shared.saving']()
              : isEditing
                ? m['billing.update_mapping']()
                : m['billing.add_provider_btn']()}
          </Button>
        </>
      }
    >
      <form
        id="provider-mapping-form"
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
      >
        <AppForm>
          <div className="space-y-4">
            {isEditing ? (
              <div className="space-y-2">
                <Label>{m['billing.label_payment_provider']()}</Label>
                <div
                  className="flex h-10 items-center rounded-md border bg-muted px-3 text-sm"
                  data-testid="provider-mapping-provider-readonly"
                >
                  {formatProviderName(mapping.paymentProvider)}
                </div>
              </div>
            ) : (
              <form.Field
                name="paymentProvider"
                children={(field) => (
                  <div className="space-y-2">
                    <Label>
                      {m['billing.label_payment_provider']()}{' '}
                      <span className="text-destructive">*</span>
                    </Label>
                    {availableProviders.length === 0 ? (
                      <div
                        className="text-sm text-muted-foreground"
                        data-testid="no-providers-message"
                      >
                        {m['billing.no_providers_realm']()}
                      </div>
                    ) : (
                      <Select
                        data-testid="provider-mapping-provider-select"
                        value={field.state.value || ''}
                        onValueChange={(value) => field.handleChange(value)}
                      >
                        <SelectTrigger data-testid="provider-mapping-provider-select-trigger">
                          <SelectValue placeholder={m['billing.placeholder_select_provider']()} />
                        </SelectTrigger>
                        <SelectContent>
                          {availableProviders.map((provider) => (
                            <SelectItem
                              key={provider}
                              value={provider}
                              data-testid={`provider-option-${provider}`}
                            >
                              {formatProviderName(provider)}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    )}
                    {(field.state.meta.isTouched || form.state.isSubmitted) &&
                      field.state.meta.errors.length > 0 && (
                        <p className="text-sm text-destructive">
                          {getFieldErrorMessage(field.state.meta)}
                        </p>
                      )}
                  </div>
                )}
              />
            )}

            <TextField
              form={form}
              name="externalProductId"
              label={m['billing.label_external_product_id']()}
              dataTestId="provider-mapping-product-id-input"
              placeholder="prod_basic_monthly"
              required
            />

            <TextField
              form={form}
              name="externalPriceId"
              label={m['billing.label_external_price_id']()}
              dataTestId="provider-mapping-price-id-input"
              placeholder="price_12345 (optional)"
            />

            <div className="flex items-center space-x-2">
              <form.Field
                name="enabled"
                children={(field) => (
                  <>
                    <Label htmlFor="provider-enabled">{m['common.enabled']()}</Label>
                    <Switch
                      id="provider-enabled"
                      data-testid="provider-mapping-enabled-switch"
                      checked={field.state.value}
                      onCheckedChange={(checked) => field.handleChange(checked)}
                    />
                  </>
                )}
              />
              <p className="text-xs text-muted-foreground">
                {m['billing.help_provider_enabled']()}
              </p>
            </div>
          </div>
        </AppForm>
      </form>
    </BaseFormDialog>
  )
}
