import { useEffect, useMemo } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import {
  shopifyConfigSchema,
  type ShopifyConfigForm,
  getShopifyConfigDefaults,
  WEBHOOK_MODES,
  type WebhookMode,
} from '@/lib/schemas/billing-forms'
import { requireFieldOnCreate } from '@/lib/form-utils'
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
import { PageHeader } from '@/components/shared/page-header'
import { FormActionBar } from '@/components/shared/form-action-bar'
import { Loader2 } from 'lucide-react'
import {
  createShopifyConfig,
  updateShopifyConfig,
  testShopifyConnection,
} from '@/lib/api-generated'

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
              required={mode !== 'edit'}
              helpText={
                mode === 'edit'
                  ? 'Leave empty to keep the existing token'
                  : 'Must start with shpat_. Used for Admin API calls.'
              }
            />

            <PasswordField
              form={form}
              name="storefrontAccessToken"
              label="Storefront Access Token"
              dataTestId="storefront-access-token-input"
              placeholder="shp_..."
              required={mode !== 'edit'}
              helpText={
                mode === 'edit'
                  ? 'Leave empty to keep the existing token'
                  : 'Must start with shp_. Used for Storefront API calls.'
              }
            />

            <PasswordField
              form={form}
              name="appClientSecret"
              label="App Client Secret"
              dataTestId="app-client-secret-input"
              placeholder="Your app client secret"
              required={mode !== 'edit'}
              helpText={
                mode === 'edit'
                  ? 'Leave empty to keep the existing secret'
                  : 'Used for webhook HMAC verification. Keep this secure!'
              }
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

interface ShopifyConfigFormPageProps {
  realmId: string
  mode: 'create' | 'edit'
  initialValues?: Partial<ShopifyConfigForm>
}

export function ShopifyConfigFormPage({
  realmId,
  mode,
  initialValues,
}: ShopifyConfigFormPageProps) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const isEditing = mode === 'edit'

  const defaultValues = useMemo(() => getShopifyConfigDefaults(initialValues), [initialValues])

  const form = useAppForm({
    schema: shopifyConfigSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      if (!requireFieldOnCreate(form, isEditing, 'adminAccessToken', value.adminAccessToken, 'Admin Access Token is required')) return
      if (!requireFieldOnCreate(form, isEditing, 'storefrontAccessToken', value.storefrontAccessToken, 'Storefront Access Token is required')) return
      if (!requireFieldOnCreate(form, isEditing, 'appClientSecret', value.appClientSecret, 'App Client Secret is required')) return
      if (isEditing) {
        await updateMutation.mutateAsync(value)
      } else {
        await createMutation.mutateAsync(value)
      }
    },
  })

  useEffect(() => {
    form.reset(defaultValues)
  }, [defaultValues, form])

  const createMutation = useMutation({
    mutationFn: async (data: ShopifyConfigForm) => {
      const response = await createShopifyConfig({
        path: { realmId },
        body: {
          shopDomain: data.shopDomain,
          adminAccessToken: data.adminAccessToken,
          storefrontAccessToken: data.storefrontAccessToken,
          appClientSecret: data.appClientSecret,
          apiVersion: data.apiVersion,
          webhookSubscriptionMode: data.webhookSubscriptionMode as 'admin_api' | 'event_bridge',
          timeout: data.timeout,
          skipConnectionTest: data.skipConnectionTest,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success('Shopify configuration created successfully')
      await queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      if (error?.status === 409) {
        toast.error('A Shopify configuration already exists. Please edit the existing one.')
      } else if (error?.status === 422) {
        toast.error('Connection test failed. Please check your credentials.')
      } else {
        toast.error(`Failed to create configuration: ${error?.message || 'Unknown error'}`)
      }
    },
  })

  const updateMutation = useMutation({
    mutationFn: async (data: ShopifyConfigForm) => {
      const body: Record<string, unknown> = {
        shopDomain: data.shopDomain,
        apiVersion: data.apiVersion,
        webhookSubscriptionMode: data.webhookSubscriptionMode as 'admin_api' | 'event_bridge',
        timeout: data.timeout,
        skipConnectionTest: data.skipConnectionTest,
      }
      if (data.adminAccessToken) body.adminAccessToken = data.adminAccessToken
      if (data.storefrontAccessToken) body.storefrontAccessToken = data.storefrontAccessToken
      if (data.appClientSecret) body.appClientSecret = data.appClientSecret
      const response = await updateShopifyConfig({
        path: { realmId },
        body: body as Parameters<typeof updateShopifyConfig>[0]['body'],
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success('Shopify configuration updated successfully')
      await queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      if (error?.status === 422) {
        toast.error('Connection test failed. Please check your credentials.')
      } else {
        toast.error(`Failed to update configuration: ${error?.message || 'Unknown error'}`)
      }
    },
  })

  const testMutation = useMutation({
    mutationFn: async (data: ShopifyConfigForm) => {
      const response = await testShopifyConnection({
        path: { realmId },
        body: {
          shopDomain: data.shopDomain,
          adminAccessToken: data.adminAccessToken,
          storefrontAccessToken: data.storefrontAccessToken,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (data) => {
      if (data?.success) {
        toast.success('Shopify connection test passed')
      } else {
        const errors = data?.errors?.join(', ') || 'Connection test failed'
        toast.error(`Connection test failed: ${errors}`)
      }
    },
    onError: (error: { message?: string }) => {
      toast.error(`Connection test failed: ${error?.message || 'Unknown error'}`)
    },
  })

  const handleTestConnection = async () => {
    await form.validateAllFields('submit')
    if (!form.state.isFieldsValid) {
      return
    }
    testMutation.mutate(form.state.values)
  }

  const handleCancel = () => {
    navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
  }

  const isSubmitting = createMutation.isPending || updateMutation.isPending
  const isTesting = testMutation.isPending

  return (
    <div className="container max-w-3xl mx-auto py-6 px-6" data-testid="shopify-config-form-page">
      <PageHeader
        title={isEditing ? 'Edit Shopify Configuration' : 'Configure Shopify'}
        headingTestId="shopify-config-form-page-heading"
      />

      <form
        onSubmit={async (e) => {
          e.preventDefault()
          e.stopPropagation()
          await form.validateAllFields('submit')
          if (!form.state.isFieldsValid) {
            return
          }
          await form.handleSubmit()
        }}
        data-testid="shopify-config-page-form"
        className="space-y-6 pt-6"
      >
        <AppForm>
          <div className="space-y-6">
            <TextField
              form={form}
              name="shopDomain"
              label="Shop Domain"
              dataTestId="page-shop-domain-input"
              placeholder="demo-store.myshopify.com"
              required
              helpText="Must end with .myshopify.com"
            />

            <PasswordField
              form={form}
              name="adminAccessToken"
              label="Admin Access Token"
              dataTestId="page-admin-access-token-input"
              placeholder="shpat_..."
              required={!isEditing}
              helpText={
                isEditing
                  ? 'Leave empty to keep the existing token'
                  : 'Must start with shpat_. Used for Admin API calls.'
              }
            />

            <PasswordField
              form={form}
              name="storefrontAccessToken"
              label="Storefront Access Token"
              dataTestId="page-storefront-access-token-input"
              placeholder="shp_..."
              required={!isEditing}
              helpText={
                isEditing
                  ? 'Leave empty to keep the existing token'
                  : 'Must start with shp_. Used for Storefront API calls.'
              }
            />

            <PasswordField
              form={form}
              name="appClientSecret"
              label="App Client Secret"
              dataTestId="page-app-client-secret-input"
              placeholder="Your app client secret"
              required={!isEditing}
              helpText={
                isEditing
                  ? 'Leave empty to keep the existing secret'
                  : 'Used for webhook HMAC verification. Keep this secure!'
              }
            />

            <TextField
              form={form}
              name="apiVersion"
              label="API Version"
              dataTestId="page-api-version-input"
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
                    data-testid="page-webhook-subscription-mode-select"
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
              dataTestId="page-timeout-input"
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
                    data-testid="page-skip-connection-test-checkbox"
                  />
                  <Label htmlFor={field.name} className="text-sm font-normal">
                    Skip connection test (for demo/test environments)
                  </Label>
                </div>
              )}
            />
          </div>
        </AppForm>

        <FormActionBar
          onCancel={handleCancel}
          isSubmitting={isSubmitting || isTesting}
          isEditing={isEditing}
          cancelTestId="shopify-config-page-cancel-button"
          submitTestId="shopify-config-page-submit-button"
        >
          <Button
            type="button"
            variant="secondary"
            onClick={handleTestConnection}
            disabled={isTesting || isSubmitting}
            data-testid="shopify-config-page-test-connection-button"
          >
            {isTesting ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Testing...
              </>
            ) : (
              'Test Connection'
            )}
          </Button>
        </FormActionBar>
      </form>
    </div>
  )
}
