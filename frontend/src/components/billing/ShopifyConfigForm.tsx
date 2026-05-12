import { useEffect, useMemo } from 'react'
import { Button } from '@/components/ui/button'
import {
  shopifyConfigSchema,
  type ShopifyConfigForm,
  getShopifyConfigDefaults,
  WEBHOOK_MODES,
  type WebhookMode,
} from '@/lib/schemas/billing-forms'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { TextField, PasswordField, NumberField } from '@/components/shared/form-fields'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Label } from '@/components/ui/label'

interface ShopifyConfigFormDialogProps {
  initialValues?: Partial<ShopifyConfigForm>
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: ShopifyConfigForm) => void
  onTestConnection?: (data: ShopifyConfigForm) => void
  isSubmitting?: boolean
  isTesting?: boolean
  mode: 'create' | 'edit'
}

export function ShopifyConfigFormDialog({
  initialValues,
  open,
  onOpenChange,
  onSubmit,
  onTestConnection,
  isSubmitting = false,
  isTesting = false,
  mode,
}: ShopifyConfigFormDialogProps) {
  const defaultValues = useMemo(() => getShopifyConfigDefaults(initialValues), [initialValues])

  const form = useAppForm({
    schema: shopifyConfigSchema,
    defaultValues,
    onSubmit: ({ value }) => onSubmit(value),
  })

  useEffect(() => {
    if (open) {
      form.reset(defaultValues)
    }
  }, [defaultValues, form, open])

  const handleTestConnection = async () => {
    await form.validateAllFields('submit')

    if (!form.state.isFieldsValid) {
      return
    }

    if (onTestConnection) {
      await onTestConnection(form.state.values)
    }
  }

  return (
    <BaseFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={mode === 'create' ? 'Configure Shopify' : 'Edit Shopify Configuration'}
      description={
        mode === 'create' ? 'Add Shopify as a payment provider' : 'Update Shopify configuration'
      }
      className="max-w-2xl"
      isSubmitting={isSubmitting}
      data-testid="shopify-config-form-dialog"
      footer={
        <>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="shopify-config-cancel-button"
          >
            Cancel
          </Button>
          {onTestConnection && (
            <Button
              type="button"
              variant="secondary"
              onClick={handleTestConnection}
              disabled={isTesting || isSubmitting}
              data-testid="shopify-config-test-connection-button"
            >
              {isTesting ? 'Testing...' : 'Test Connection'}
            </Button>
          )}
          <Button
            type="submit"
            form="shopify-config-form"
            disabled={isSubmitting || isTesting}
            data-testid="shopify-config-submit-button"
          >
            {isSubmitting
              ? 'Saving...'
              : mode === 'create'
                ? 'Create Configuration'
                : 'Save Changes'}
          </Button>
        </>
      }
    >
      <form
        id="shopify-config-form"
        onSubmit={async (e) => {
          e.preventDefault()
          e.stopPropagation()

          // Validate all fields before submission
          await form.validateAllFields('submit')

          if (!form.state.isFieldsValid) {
            return
          }

          await form.handleSubmit()
        }}
        data-testid="shopify-config-form"
      >
        <AppForm>
          <div className="space-y-6">
            <TextField
              form={form}
              name="shopDomain"
              label="Shop Domain"
              dataTestId="shop-domain-input"
              placeholder="demo-store.myshopify.com"
              required
              helpText="Must end with .myshopify.com"
            />

            <PasswordField
              form={form}
              name="adminAccessToken"
              label="Admin Access Token"
              dataTestId="admin-access-token-input"
              placeholder="shpat_..."
              required
              helpText="Must start with shpat_. Used for Admin API calls."
            />

            <PasswordField
              form={form}
              name="storefrontAccessToken"
              label="Storefront Access Token"
              dataTestId="storefront-access-token-input"
              placeholder="shp_..."
              required
              helpText="Must start with shp_. Used for Storefront API calls."
            />

            <PasswordField
              form={form}
              name="appClientSecret"
              label="App Client Secret"
              dataTestId="app-client-secret-input"
              placeholder="Your app client secret"
              required
              helpText="Used for webhook HMAC verification. Keep this secure!"
            />

            <TextField
              form={form}
              name="apiVersion"
              label="API Version"
              dataTestId="api-version-input"
              placeholder="2024-01"
              helpText="Shopify Admin API version (default: 2024-01)"
            />

            <form.Field
              name="webhookSubscriptionMode"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>Webhook Subscription Mode</Label>
                  <Select
                    value={field.state.value}
                    onValueChange={(value) => field.handleChange(value as WebhookMode)}
                    data-testid="webhook-subscription-mode-select"
                  >
                    <SelectTrigger id={field.name}>
                      <SelectValue placeholder="Select webhook mode" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={WEBHOOK_MODES.ADMIN_API}>Admin API</SelectItem>
                      <SelectItem value={WEBHOOK_MODES.EVENT_BRIDGE}>Event Bridge</SelectItem>
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">
                    How webhooks are delivered to your app
                  </p>
                </div>
              )}
            />

            <NumberField
              form={form}
              name="timeout"
              label="Timeout (seconds)"
              dataTestId="timeout-input"
              placeholder="30"
              min={1}
              max={120}
              helpText="HTTP request timeout in seconds (1-120, default: 30)"
            />

            <form.Field
              name="skipConnectionTest"
              children={(field) => (
                <div className="flex items-center space-x-2">
                  <input
                    type="checkbox"
                    id={field.name}
                    checked={field.state.value}
                    onChange={(e) => field.handleChange(e.target.checked)}
                    className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-600"
                    data-testid="skip-connection-test-checkbox"
                  />
                  <Label htmlFor={field.name} className="text-sm font-normal">
                    Skip connection test (for demo/test environments)
                  </Label>
                </div>
              )}
            />
          </div>
        </AppForm>
      </form>
    </BaseFormDialog>
  )
}
