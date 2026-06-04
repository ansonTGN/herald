import { useEffect, useMemo } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { queryKeys } from '@/data/query-options'
import { Button } from '@/components/ui/button'
import {
  wechatConfigSchema,
  type WechatConfigForm,
  getWechatConfigDefaults,
} from '@/lib/schemas/billing-forms'
import { requireFieldOnCreate } from '@/lib/form-utils'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { TextField, PasswordField, SwitchField } from '@/components/shared/form-fields'
import { Textarea } from '@/components/ui/textarea'
import { PageHeader } from '@/components/shared/page-header'
import { FormActionBar } from '@/components/shared/form-action-bar'
import { createWechatConfig, updateWechatConfig } from '@/lib/api-generated'
import { upsertRealmConfig } from '@/lib/api-generated/sdk.gen'
import { m } from '@/paraglide/messages'

interface WechatConfigFormDialogProps {
  initialValues?: Partial<WechatConfigForm>
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: WechatConfigForm) => void
  isSubmitting?: boolean
  mode: 'create' | 'edit'
}

export function WechatConfigFormDialog({
  initialValues,
  open,
  onOpenChange,
  onSubmit,
  isSubmitting = false,
  mode,
}: WechatConfigFormDialogProps) {
  const defaultValues = useMemo(() => getWechatConfigDefaults(initialValues), [initialValues])

  const form = useAppForm({
    schema: wechatConfigSchema,
    defaultValues,
    onSubmit: ({ value }) => onSubmit(value),
  })

  useEffect(() => {
    if (open) {
      form.reset(defaultValues)
    }
  }, [defaultValues, form, open])

  const handleSubmit = async () => {
    await form.validateAllFields('submit')

    if (!form.state.isFieldsValid) {
      return
    }

    await form.handleSubmit()
  }

  return (
    <BaseFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={mode === 'create' ? m['billing.wechat_configure']() : m['billing.wechat_edit']()}
      description={
        mode === 'create'
          ? m['billing.wechat_create_description']()
          : m['billing.wechat_edit_description']()
      }
      className="max-w-2xl"
      isSubmitting={isSubmitting}
      data-testid="wechat-config-form-dialog"
      footer={
        <>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="wechat-config-cancel-button"
          >
            {m['common.cancel']()}
          </Button>
          <Button
            type="button"
            onClick={handleSubmit}
            disabled={isSubmitting}
            data-testid="wechat-config-submit-button"
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
        id="wechat-config-form"
        onSubmit={async (e) => {
          e.preventDefault()
          e.stopPropagation()
          await handleSubmit()
        }}
        data-testid="wechat-config-form"
      >
        <AppForm>
          <div className="space-y-6">
            <TextField
              form={form}
              name="appId"
              label={m['billing.wechat_app_id']()}
              dataTestId="app-id-input"
              placeholder="wx1234567890abcdef"
              required
              helpText={m['billing.wechat_app_id_help']()}
            />

            <TextField
              form={form}
              name="mchId"
              label={m['billing.wechat_merchant_id']()}
              dataTestId="merchant-id-input"
              placeholder="1234567890"
              required
              helpText={m['billing.wechat_merchant_id_help']()}
            />

            <form.Field
              name="privateKey"
              children={(field) => (
                <div className="space-y-2">
                  <label htmlFor={field.name} className="text-sm font-medium">
                    {m['billing.wechat_private_key']()} <span className="text-destructive">*</span>
                  </label>
                  <Textarea
                    id={field.name}
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="-----BEGIN PRIVATE KEY-----&#10;...&#10;-----END PRIVATE KEY-----"
                    rows={8}
                    className="font-mono text-xs"
                    data-testid="private-key-input"
                  />
                  {field.state.meta.errors && (
                    <p className="text-sm text-destructive">
                      {field.state.meta.errors[0]?.message}
                    </p>
                  )}
                  <p className="text-xs text-muted-foreground">
                    {m['billing.wechat_private_key_help']()}
                  </p>
                </div>
              )}
            />

            <TextField
              form={form}
              name="serialNo"
              label={m['billing.wechat_serial_no']()}
              dataTestId="serial-no-input"
              placeholder="1A2B3C4D5E6F"
              required
              helpText={m['billing.wechat_serial_no_help']()}
            />

            <PasswordField
              form={form}
              name="v3Key"
              label={m['billing.wechat_v3_key']()}
              dataTestId="v3-key-input"
              placeholder="Enter 32-character API v3 key"
              required
              showToggle={false}
              helpText={m['billing.wechat_v3_key_help']()}
            />

            <form.Field
              name="platformPublicKey"
              children={(field) => (
                <div className="space-y-2">
                  <label htmlFor={field.name} className="text-sm font-medium">
                    {m['billing.wechat_platform_public_key']()}{' '}
                    <span className="text-destructive">*</span>
                  </label>
                  <Textarea
                    id={field.name}
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="-----BEGIN PUBLIC KEY-----&#10;...&#10;-----END PUBLIC KEY-----"
                    rows={5}
                    className="font-mono text-xs"
                    data-testid="platform-public-key-input"
                  />
                  {field.state.meta.errors && (
                    <p className="text-sm text-destructive">
                      {field.state.meta.errors[0]?.message}
                    </p>
                  )}
                  <p className="text-xs text-muted-foreground">
                    {m['billing.wechat_platform_public_key_help']()}
                  </p>
                </div>
              )}
            />

            <TextField
              form={form}
              name="notifyUrl"
              label={m['billing.wechat_notify_url']()}
              dataTestId="notify-url-input"
              placeholder="https://api.example.com/api/third/pay/{realmId}/wechat/webhooks"
              required
              helpText={m['billing.wechat_notify_url_help']()}
            />
          </div>
        </AppForm>
      </form>
    </BaseFormDialog>
  )
}

interface WechatConfigFormPageProps {
  realmId: string
  mode: 'create' | 'edit'
  initialValues?: Partial<WechatConfigForm>
}

export function WechatConfigFormPage({ realmId, mode, initialValues }: WechatConfigFormPageProps) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const isEditing = mode === 'edit'

  const defaultValues = useMemo(() => getWechatConfigDefaults(initialValues), [initialValues])

  const form = useAppForm({
    schema: wechatConfigSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      if (
        !requireFieldOnCreate(
          form,
          isEditing,
          'privateKey',
          value.privateKey,
          m['billing.wechat_private_key_required']()
        )
      )
        return
      if (
        !requireFieldOnCreate(
          form,
          isEditing,
          'v3Key',
          value.v3Key,
          m['billing.wechat_v3_key_required']()
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
    mutationFn: async (data: WechatConfigForm) => {
      const response = await createWechatConfig({
        path: { realmId },
        body: {
          appId: data.appId,
          mchId: data.mchId,
          notifyUrl: data.notifyUrl,
          privateKey: data.privateKey,
          serialNo: data.serialNo,
          v3Key: data.v3Key,
          platformPublicKey: data.platformPublicKey,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success(m['billing.wechat_created']())
      await queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      await queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      if (error?.status === 409) {
        toast.error(m['billing.wechat_conflict']())
      } else {
        toast.error(
          m['billing.wechat_create_failed']({ message: error?.message || 'Unknown error' })
        )
      }
    },
  })

  const updateMutation = useMutation({
    mutationFn: async (data: WechatConfigForm) => {
      const body: Record<string, unknown> = {
        appId: data.appId,
        mchId: data.mchId,
        notifyUrl: data.notifyUrl,
        serialNo: data.serialNo,
      }
      if (data.privateKey) body.privateKey = data.privateKey
      if (data.v3Key) body.v3Key = data.v3Key
      if (data.platformPublicKey) body.platformPublicKey = data.platformPublicKey
      const response = await updateWechatConfig({
        path: { realmId },
        body: body as Parameters<typeof updateWechatConfig>[0]['body'],
      })
      if (response.error) throw response.error

      await upsertRealmConfig({
        path: { realmId },
        body: {
          configType: 'wechat',
          configKey: 'enabled',
          configValue: String(data.enabled),
          enabled: data.enabled,
        },
      })

      return response.data
    },
    onSuccess: async () => {
      toast.success(m['billing.wechat_updated']())
      await queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      await queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      toast.error(m['billing.wechat_update_failed']({ message: error?.message || 'Unknown error' }))
    },
  })

  const handleCancel = () => {
    navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
  }

  const isSubmitting = createMutation.isPending || updateMutation.isPending

  return (
    <div className="container max-w-3xl mx-auto py-6 px-6" data-testid="wechat-config-form-page">
      <PageHeader
        title={isEditing ? m['billing.wechat_edit']() : m['billing.wechat_configure']()}
        headingTestId="wechat-config-form-page-heading"
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
        data-testid="wechat-config-page-form"
        className="space-y-6 pt-6"
      >
        <AppForm>
          <div className="space-y-6">
            {isEditing && (
              <SwitchField
                form={form}
                name="enabled"
                label={m['billing.wechat_enable']()}
                description={m['billing.wechat_enable_description']()}
                dataTestId="page-wechat-enabled-switch"
              />
            )}

            <TextField
              form={form}
              name="appId"
              label={m['billing.wechat_app_id']()}
              dataTestId="page-app-id-input"
              placeholder="Enter WeChat App ID (e.g. wx1234567890)"
              required
              helpText={m['billing.wechat_app_id_help']()}
            />

            <TextField
              form={form}
              name="mchId"
              label={m['billing.wechat_merchant_id']()}
              dataTestId="page-merchant-id-input"
              placeholder="Enter merchant ID"
              required
              helpText={m['billing.wechat_merchant_id_help']()}
            />

            <form.Field
              name="privateKey"
              children={(field) => (
                <div className="space-y-2">
                  <label htmlFor={field.name} className="text-sm font-medium">
                    {m['billing.wechat_private_key']()}{' '}
                    {!isEditing && <span className="text-destructive">*</span>}
                  </label>
                  <Textarea
                    id={field.name}
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="-----BEGIN PRIVATE KEY-----&#10;...&#10;-----END PRIVATE KEY-----"
                    rows={8}
                    className="font-mono text-xs"
                    data-testid="page-private-key-input"
                  />
                  {field.state.meta.errors && (
                    <p className="text-sm text-destructive">
                      {field.state.meta.errors[0]?.message}
                    </p>
                  )}
                  <p className="text-xs text-muted-foreground">
                    {isEditing
                      ? m['billing.wechat_private_key_keep']()
                      : m['billing.wechat_private_key_help']()}
                  </p>
                </div>
              )}
            />

            <TextField
              form={form}
              name="serialNo"
              label={m['billing.wechat_serial_no']()}
              dataTestId="page-serial-no-input"
              placeholder="Enter certificate serial number"
              required
              helpText={m['billing.wechat_serial_no_help']()}
            />

            <PasswordField
              form={form}
              name="v3Key"
              label={m['billing.wechat_v3_key']()}
              dataTestId="page-v3-key-input"
              placeholder="Enter 32-character API v3 key"
              required={!isEditing}
              showToggle={false}
              helpText={
                isEditing ? m['billing.wechat_v3_key_keep']() : m['billing.wechat_v3_key_help']()
              }
            />

            <TextField
              form={form}
              name="notifyUrl"
              label={m['billing.wechat_notify_url']()}
              dataTestId="page-notify-url-input"
              placeholder="https://api.example.com/api/third/pay/{realmId}/wechat/webhooks"
              required
              helpText={m['billing.wechat_notify_url_help']()}
            />
          </div>
        </AppForm>

        <FormActionBar
          onCancel={handleCancel}
          isSubmitting={isSubmitting}
          isEditing={isEditing}
          cancelTestId="wechat-config-page-cancel-button"
          submitTestId="wechat-config-page-submit-button"
        />
      </form>
    </div>
  )
}
