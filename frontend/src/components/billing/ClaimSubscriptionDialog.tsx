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
      title="Claim Shopify Subscription"
      description="Enter your Shopify information to claim your subscription"
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
            {isSubmitting ? 'Claiming...' : 'Claim Subscription'}
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
              <Label>Shopify Customer ID</Label>
              <TextField
                form={form}
                name="shopifyCustomerId"
                label=""
                dataTestId="shopify-customer-id-input"
                placeholder="customer_123"
                helpText="Found in Shopify Admin → Customers. Click on the customer to see their ID."
              />
            </div>

            <div className="relative">
              <div className="absolute inset-0 flex items-center" aria-hidden="true">
                <div className="w-full border-t" />
              </div>
              <div className="relative flex justify-center text-sm font-medium">
                <span className="bg-background px-2 text-muted-foreground">OR</span>
              </div>
            </div>

            <div className="space-y-2">
              <Label>Subscription Contract ID</Label>
              <TextField
                form={form}
                name="contractId"
                label=""
                dataTestId="contract-id-input"
                placeholder="gid://shopify/SubscriptionContract/..."
                helpText="Found in Shopify email or account settings. Format: gid://shopify/SubscriptionContract/..."
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
                      Grant current period points
                    </Label>
                    <p className="text-xs text-muted-foreground">
                      If enabled, you'll receive points for the current billing period of your
                      subscription
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
