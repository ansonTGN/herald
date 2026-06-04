import { useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  claimSubscriptionSchema,
  type ClaimSubscriptionForm,
  getClaimSubscriptionDefaults,
} from '@/lib/schemas/billing-forms'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { TextField } from '@/components/shared/form-fields'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { Label } from '@/components/ui/label'
import { m } from '@/paraglide/messages'

interface ClaimSubscriptionDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: ClaimSubscriptionForm) => void
  isSubmitting?: boolean
}

export function ClaimSubscriptionDialog({
  open,
  onOpenChange,
  onSubmit,
  isSubmitting = false,
}: ClaimSubscriptionDialogProps) {
  const defaultValues = getClaimSubscriptionDefaults()

  const form = useAppForm({
    schema: claimSubscriptionSchema,
    defaultValues,
    onSubmit: ({ value }) => onSubmit(value),
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
      title={m['billing.claim_dialog_title']()}
      description={m['billing.claim_dialog_description']()}
      className="max-w-lg"
      isSubmitting={isSubmitting}
      data-testid="claim-subscription-dialog"
      footer={
        <>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="claim-cancel-button"
          >
            Cancel
          </Button>
          <Button
            type="submit"
            form="claim-subscription-form"
            disabled={isSubmitting}
            data-testid="claim-submit-button"
          >
            {isSubmitting ? m['billing.claim_claiming']() : m['billing.claim_claim_button']()}
          </Button>
        </>
      }
    >
      <form
        id="claim-subscription-form"
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
        data-testid="claim-subscription-form"
      >
        <AppForm>
          <div className="space-y-6">
            <div className="space-y-2">
              <Label>{m['billing.claim_shopify_customer_id']()}</Label>
              <TextField
                form={form}
                name="shopifyCustomerId"
                label=""
                dataTestId="shopify-customer-id-input"
                placeholder="customer_123"
                helpText={m['billing.claim_shopify_customer_id_help']()}
              />
            </div>

            <div className="relative">
              <div className="absolute inset-0 flex items-center" aria-hidden="true">
                <div className="w-full border-t" />
              </div>
              <div className="relative flex justify-center text-sm font-medium">
                <span className="bg-background px-2 text-muted-foreground">
                  {m['billing.claim_or']()}
                </span>
              </div>
            </div>

            <div className="space-y-2">
              <Label>{m['billing.claim_contract_id']()}</Label>
              <TextField
                form={form}
                name="contractId"
                label=""
                dataTestId="contract-id-input"
                placeholder="gid://shopify/SubscriptionContract/..."
                helpText={m['billing.claim_contract_id_help']()}
              />
            </div>

            <form.Field
              name="grantCurrentPeriod"
              children={(field) => (
                <div className="flex items-start space-x-3 space-y-0">
                  <Checkbox
                    id={field.name}
                    data-testid="grant-current-period-checkbox"
                    checked={field.state.value}
                    onCheckedChange={(checked) => field.handleChange(checked as boolean)}
                  />
                  <div className="grid gap-1.5 leading-none">
                    <Label htmlFor={field.name} className="font-normal">
                      {m['billing.claim_grant_period_points']()}
                    </Label>
                    <p className="text-xs text-muted-foreground">
                      {m['billing.claim_grant_period_points_help']()}
                    </p>
                  </div>
                </div>
              )}
            />

            <form.Field
              name="shopifyCustomerId"
              children={(field) =>
                field.state.meta.errors.length > 0 && (
                  <p className="text-sm text-destructive">
                    {getFieldErrorMessage(field.state.meta)}
                  </p>
                )
              }
            />
          </div>
        </AppForm>
      </form>
    </BaseFormDialog>
  )
}
