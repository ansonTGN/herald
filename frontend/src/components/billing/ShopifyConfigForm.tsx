import { useEffect, useMemo } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { queryKeys } from '@/data/query-options'
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
import { m } from '@/paraglide/messages'

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
      title={mode === 'create' ? m['billing.shopify_configure']() : m['billing.shopify_edit']()}
      description={
        mode === 'create'
          ? m['billing.shopify_create_description']()
          : m['billing.shopify_edit_description']()
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
            {m['common.cancel']()}
          </Button>
          {onTestConnection && (
            <Button
              type="button"
              variant="secondary"
              onClick={handleTestConnection}
              disabled={isTesting || isSubmitting}
              data-testid="shopify-config-test-connection-button"
            >
              {isTesting ? m['billing.shopify_testing']() : m['billing.shopify_test_connection']()}
            </Button>
          )}
          <Button
            type="submit"
            form="shopify-config-form"
            disabled={isSubmitting || isTesting}
            data-testid="shopify-config-submit-button"
          >
            {isSubmitting
              ? m['shared.saving']()
              : mode === 'create'
                ? m['shared.create_configuration']()
                : m['shared.save_changes']()}
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
              label={m['billing.shopify_label_shop_domain']()}
              dataTestId="shop-domain-input"
              placeholder="demo-store.myshopify.com"
              required
              helpText={m['billing.shopify_help_shop_domain']()}
            />

            <PasswordField
              form={form}
              name="adminAccessToken"
              label={m['billing.shopify_label_admin_token']()}
              dataTestId="admin-access-token-input"
              placeholder="shpat_..."
              required={mode !== 'edit'}
              helpText={
                mode === 'edit'
                  ? m['billing.shopify_help_admin_token_edit']()
                  : m['billing.shopify_help_admin_token_create']()
              }
            />

            <PasswordField
              form={form}
              name="storefrontAccessToken"
              label={m['billing.shopify_label_storefront_token']()}
              dataTestId="storefront-access-token-input"
              placeholder="shp_..."
              required={mode !== 'edit'}
              helpText={
                mode === 'edit'
                  ? m['billing.shopify_help_storefront_token_edit']()
                  : m['billing.shopify_help_storefront_token_create']()
              }
            />

            <PasswordField
              form={form}
              name="appClientSecret"
              label={m['billing.shopify_label_app_secret']()}
              dataTestId="app-client-secret-input"
              placeholder="Your app client secret"
              required={mode !== 'edit'}
              helpText={
                mode === 'edit'
                  ? m['billing.shopify_help_app_secret_edit']()
                  : m['billing.shopify_help_app_secret_create']()
              }
            />

            <TextField
              form={form}
              name="apiVersion"
              label={m['billing.shopify_label_api_version']()}
              dataTestId="api-version-input"
              placeholder="2024-01"
              helpText={m['billing.shopify_help_api_version']()}
            />

            <form.Field
              name="webhookSubscriptionMode"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>{m['billing.shopify_label_webhook_mode']()}</Label>
                  <Select
                    value={field.state.value}
                    onValueChange={(value) => field.handleChange(value as WebhookMode)}
                    data-testid="webhook-subscription-mode-select"
                  >
                    <SelectTrigger id={field.name}>
                      <SelectValue placeholder={m['billing.shopify_label_webhook_mode']()} />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={WEBHOOK_MODES.ADMIN_API}>
                        {m['billing.shopify_webhook_admin_api']()}
                      </SelectItem>
                      <SelectItem value={WEBHOOK_MODES.EVENT_BRIDGE}>
                        {m['billing.shopify_webhook_event_bridge']()}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">
                    {m['billing.shopify_help_webhook_mode']()}
                  </p>
                </div>
              )}
            />

            <NumberField
              form={form}
              name="timeout"
              label={m['billing.shopify_label_timeout']()}
              dataTestId="timeout-input"
              placeholder="30"
              min={1}
              max={120}
              helpText={m['billing.shopify_help_timeout']()}
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
                    {m['billing.shopify_skip_connection_test']()}
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
      if (
        !requireFieldOnCreate(
          form,
          isEditing,
          'adminAccessToken',
          value.adminAccessToken,
          m['billing.shopify_label_admin_token']()
        )
      )
        return
      if (
        !requireFieldOnCreate(
          form,
          isEditing,
          'storefrontAccessToken',
          value.storefrontAccessToken,
          m['billing.shopify_label_storefront_token']()
        )
      )
        return
      if (
        !requireFieldOnCreate(
          form,
          isEditing,
          'appClientSecret',
          value.appClientSecret,
          m['billing.shopify_label_app_secret']()
        )
      )
        return
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
      toast.success(m['billing.shopify_created']())
      await queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      await queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      if (error?.status === 409) {
        toast.error(m['billing.shopify_conflict']())
      } else if (error?.status === 422) {
        toast.error(m['billing.shopify_credentials_failed']())
      } else {
        toast.error(
          m['billing.shopify_create_failed']({ message: error?.message || 'Unknown error' })
        )
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
      toast.success(m['billing.shopify_updated']())
      await queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      await queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      if (error?.status === 422) {
        toast.error(m['billing.shopify_credentials_failed']())
      } else {
        toast.error(
          m['billing.shopify_update_failed']({ message: error?.message || 'Unknown error' })
        )
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
        toast.success(m['billing.shopify_connection_passed']())
      } else {
        const errors = data?.errors?.join(', ') || 'Connection test failed'
        toast.error(m['billing.shopify_connection_failed']({ errors }))
      }
    },
    onError: (error: { message?: string }) => {
      toast.error(
        m['billing.shopify_connection_failed_generic']({
          message: error?.message || 'Unknown error',
        })
      )
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
        title={isEditing ? m['billing.shopify_edit']() : m['billing.shopify_configure']()}
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
              label={m['billing.shopify_label_shop_domain']()}
              dataTestId="page-shop-domain-input"
              placeholder="demo-store.myshopify.com"
              required
              helpText={m['billing.shopify_help_shop_domain']()}
            />

            <PasswordField
              form={form}
              name="adminAccessToken"
              label={m['billing.shopify_label_admin_token']()}
              dataTestId="page-admin-access-token-input"
              placeholder="shpat_..."
              required={!isEditing}
              helpText={
                isEditing
                  ? m['billing.shopify_help_admin_token_edit']()
                  : m['billing.shopify_help_admin_token_create']()
              }
            />

            <PasswordField
              form={form}
              name="storefrontAccessToken"
              label={m['billing.shopify_label_storefront_token']()}
              dataTestId="page-storefront-access-token-input"
              placeholder="shp_..."
              required={!isEditing}
              helpText={
                isEditing
                  ? m['billing.shopify_help_storefront_token_edit']()
                  : m['billing.shopify_help_storefront_token_create']()
              }
            />

            <PasswordField
              form={form}
              name="appClientSecret"
              label={m['billing.shopify_label_app_secret']()}
              dataTestId="page-app-client-secret-input"
              placeholder="Your app client secret"
              required={!isEditing}
              helpText={
                isEditing
                  ? m['billing.shopify_help_app_secret_edit']()
                  : m['billing.shopify_help_app_secret_create']()
              }
            />

            <TextField
              form={form}
              name="apiVersion"
              label={m['billing.shopify_label_api_version']()}
              dataTestId="page-api-version-input"
              placeholder="2024-01"
              helpText={m['billing.shopify_help_api_version']()}
            />

            <form.Field
              name="webhookSubscriptionMode"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor={field.name}>{m['billing.shopify_label_webhook_mode']()}</Label>
                  <Select
                    value={field.state.value}
                    onValueChange={(value) => field.handleChange(value as WebhookMode)}
                    data-testid="page-webhook-subscription-mode-select"
                  >
                    <SelectTrigger id={field.name}>
                      <SelectValue placeholder={m['billing.shopify_label_webhook_mode']()} />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={WEBHOOK_MODES.ADMIN_API}>
                        {m['billing.shopify_webhook_admin_api']()}
                      </SelectItem>
                      <SelectItem value={WEBHOOK_MODES.EVENT_BRIDGE}>
                        {m['billing.shopify_webhook_event_bridge']()}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                  <p className="text-xs text-muted-foreground">
                    {m['billing.shopify_help_webhook_mode']()}
                  </p>
                </div>
              )}
            />

            <NumberField
              form={form}
              name="timeout"
              label={m['billing.shopify_label_timeout']()}
              dataTestId="page-timeout-input"
              placeholder="30"
              min={1}
              max={120}
              helpText={m['billing.shopify_help_timeout']()}
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
                    {m['billing.shopify_skip_connection_test']()}
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
                {m['billing.shopify_testing']()}
              </>
            ) : (
              m['billing.shopify_test_connection']()
            )}
          </Button>
        </FormActionBar>
      </form>
    </div>
  )
}
