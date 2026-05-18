import { useEffect, useMemo } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import {
  wechatConfigSchema,
  type WechatConfigForm,
  getWechatConfigDefaults,
} from '@/lib/schemas/billing-forms'
import { requireFieldOnCreate } from '@/lib/form-utils'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { TextField, PasswordField } from '@/components/shared/form-fields'
import { Textarea } from '@/components/ui/textarea'
import { PageHeader } from '@/components/shared/page-header'
import { FormActionBar } from '@/components/shared/form-action-bar'
import { createWechatConfig, updateWechatConfig } from '@/lib/api-generated'

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
      title={mode === 'create' ? 'Configure WeChat Pay' : 'Edit WeChat Pay Configuration'}
      description={
        mode === 'create'
          ? 'Add WeChat Pay as a payment provider'
          : 'Update WeChat Pay configuration'
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
            Cancel
          </Button>
          <Button
            type="button"
            onClick={handleSubmit}
            disabled={isSubmitting}
            data-testid="wechat-config-submit-button"
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
              label="App ID"
              dataTestId="app-id-input"
              placeholder="wx1234567890abcdef"
              required
              helpText="Must start with wx. Available in your WeChat Pay merchant account."
            />

            <TextField
              form={form}
              name="mchId"
              label="Merchant ID"
              dataTestId="merchant-id-input"
              placeholder="1234567890"
              required
              helpText="Numeric merchant ID from your WeChat Pay account."
            />

            <form.Field
              name="privateKey"
              children={(field) => (
                <div className="space-y-2">
                  <label htmlFor={field.name} className="text-sm font-medium">
                    Private Key <span className="text-destructive">*</span>
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
                    Merchant private key in PEM format. Used for signing requests to WeChat Pay API.
                  </p>
                </div>
              )}
            />

            <TextField
              form={form}
              name="serialNo"
              label="Serial No"
              dataTestId="serial-no-input"
              placeholder="1A2B3C4D5E6F"
              required
              helpText="Certificate serial number from your WeChat Pay merchant account."
            />

            <PasswordField
              form={form}
              name="v3Key"
              label="API v3 Key"
              dataTestId="v3-key-input"
              placeholder="0123456789abcdef0123456789abcdef"
              required
              helpText="Exactly 32 characters. Used for verifying webhook signatures."
            />

            <TextField
              form={form}
              name="notifyUrl"
              label="Notify URL"
              dataTestId="notify-url-input"
              placeholder="https://api.example.com/api/third/pay/{realmId}/wechat/webhooks"
              required
              helpText="Must use HTTPS. WeChat Pay will send payment notifications to this URL."
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
      if (!requireFieldOnCreate(form, isEditing, 'privateKey', value.privateKey, 'Private Key is required')) return
      if (!requireFieldOnCreate(form, isEditing, 'v3Key', value.v3Key, 'API v3 Key is required')) return
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
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success('WeChat Pay configuration created successfully')
      await queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      if (error?.status === 409) {
        toast.error('A WeChat Pay configuration already exists. Please edit the existing one.')
      } else {
        toast.error(`Failed to create configuration: ${error?.message || 'Unknown error'}`)
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
      const response = await updateWechatConfig({
        path: { realmId },
        body: body as Parameters<typeof updateWechatConfig>[0]['body'],
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success('WeChat Pay configuration updated successfully')
      await queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] })
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      toast.error(`Failed to update configuration: ${error?.message || 'Unknown error'}`)
    },
  })

  const handleCancel = () => {
    navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
  }

  const isSubmitting = createMutation.isPending || updateMutation.isPending

  return (
    <div className="container max-w-3xl mx-auto py-6 px-6" data-testid="wechat-config-form-page">
      <PageHeader
        title={isEditing ? 'Edit WeChat Pay Configuration' : 'Configure WeChat Pay'}
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
            <TextField
              form={form}
              name="appId"
              label="App ID"
              dataTestId="page-app-id-input"
              placeholder="wx1234567890abcdef"
              required
              helpText="Must start with wx. Available in your WeChat Pay merchant account."
            />

            <TextField
              form={form}
              name="mchId"
              label="Merchant ID"
              dataTestId="page-merchant-id-input"
              placeholder="1234567890"
              required
              helpText="Numeric merchant ID from your WeChat Pay account."
            />

            <form.Field
              name="privateKey"
              children={(field) => (
                <div className="space-y-2">
                  <label htmlFor={field.name} className="text-sm font-medium">
                    Private Key {!isEditing && <span className="text-destructive">*</span>}
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
                      ? 'Leave empty to keep the existing key'
                      : 'Merchant private key in PEM format. Used for signing requests to WeChat Pay API.'}
                  </p>
                </div>
              )}
            />

            <TextField
              form={form}
              name="serialNo"
              label="Serial No"
              dataTestId="page-serial-no-input"
              placeholder="1A2B3C4D5E6F"
              required
              helpText="Certificate serial number from your WeChat Pay merchant account."
            />

            <PasswordField
              form={form}
              name="v3Key"
              label="API v3 Key"
              dataTestId="page-v3-key-input"
              placeholder="0123456789abcdef0123456789abcdef"
              required={!isEditing}
              helpText={
                isEditing
                  ? 'Leave empty to keep the existing key'
                  : 'Exactly 32 characters. Used for verifying webhook signatures.'
              }
            />

            <TextField
              form={form}
              name="notifyUrl"
              label="Notify URL"
              dataTestId="page-notify-url-input"
              placeholder="https://api.example.com/api/third/pay/{realmId}/wechat/webhooks"
              required
              helpText="Must use HTTPS. WeChat Pay will send payment notifications to this URL."
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
