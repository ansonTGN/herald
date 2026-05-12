import { useEffect, useMemo, useRef } from 'react'
import { Button } from '@/components/ui/button'
import {
  wechatConfigSchema,
  type WechatConfigForm,
  getWechatConfigDefaults,
} from '@/lib/schemas/billing-forms'
import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import { BaseFormDialog } from '@/components/shared/form-dialog'
import { TextField, PasswordField } from '@/components/shared/form-fields'
import { Textarea } from '@/components/ui/textarea'

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

  const formRef = useRef<HTMLFormElement>(null)

  useEffect(() => {
    if (open) {
      form.reset(defaultValues)
    }
  }, [defaultValues, form, open])

  const handleSubmit = async () => {
    // Validate all fields before submission
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
        ref={formRef}
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
