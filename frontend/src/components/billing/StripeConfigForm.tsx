import { useEffect, useMemo, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  stripeConfigSchema,
  type StripeConfigForm as StripeConfigFormValues,
  getStripeConfigDefaults,
} from '@/lib/schemas/stripe-config'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { PageHeader } from '@/components/shared/page-header'
import { FormActionBar } from '@/components/shared/form-action-bar'
import { SwitchField, PasswordField } from '@/components/shared/form-fields'
import { getFieldErrorMessage } from '@/lib/error-utils'
import { batchUpsertRealmConfigs } from '@/lib/api-generated/sdk.gen'
import { buildStripeConfigRequest } from '@/lib/stripe-config-utils'
import { requireFieldOnCreate } from '@/lib/form-utils'
import { useSaveConfigMutation } from '@/hooks/use-save-config-mutation'
import { m } from '@/paraglide/messages'

interface StripeConfigFormDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  initialValues: Partial<StripeConfigFormValues>
  onSubmit: (config: StripeConfigFormValues) => Promise<void>
  isSubmitting?: boolean
  mode: 'create' | 'edit'
}

export function StripeConfigFormDialog({
  open,
  onOpenChange,
  initialValues,
  onSubmit,
  mode,
}: StripeConfigFormDialogProps) {
  const [hasChanges, setHasChanges] = useState(false)

  const form = useAppForm({
    schema: stripeConfigSchema,
    defaultValues: { ...getStripeConfigDefaults(), ...initialValues },
    onSubmit: async ({ value }) => {
      try {
        await onSubmit(value)
        onOpenChange(false)
        setHasChanges(false)
      } catch (error) {
        console.error('Failed to save Stripe configuration:', error)
        throw error
      }
    },
  })

  return (
    <Dialog
      open={open}
      onOpenChange={(open) => {
        if (!open || !hasChanges || confirm(m['billing.unsaved_changes']())) {
          onOpenChange(open)
          if (!open) setHasChanges(false)
        }
      }}
    >
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            {mode === 'create' ? m['billing.configure_stripe']() : m['billing.edit_stripe']()}
          </DialogTitle>
          <DialogDescription>{m['billing.stripe_description']()}</DialogDescription>
        </DialogHeader>

        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              form.handleSubmit()
            }}
            className="space-y-4"
          >
            <form.Field
              name="enabled"
              children={(field) => (
                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label htmlFor="stripe-enabled">{m['billing.stripe_enable']()}</Label>
                    <p className="text-sm text-muted-foreground">
                      {m['billing.stripe_enable_description']()}
                    </p>
                  </div>
                  <Switch
                    id="stripe-enabled"
                    checked={field.state.value}
                    onCheckedChange={(checked) => {
                      field.handleChange(checked)
                      setHasChanges(true)
                    }}
                    data-testid="stripe-enabled-switch"
                  />
                </div>
              )}
            />

            <form.Field
              name="publishableKey"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="stripe-publishable-key">
                    {m['billing.stripe_publishable_key']()}
                  </Label>
                  <Input
                    id="stripe-publishable-key"
                    type="password"
                    value={field.state.value}
                    onChange={(e) => {
                      field.handleChange(e.target.value)
                      setHasChanges(true)
                    }}
                    placeholder="pk_test_..."
                    data-testid="stripe-publishable-key-input"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-red-500">
                        {getFieldErrorMessage(field.state.meta.errors[0])}
                      </p>
                    )}
                  <p className="text-xs text-muted-foreground">
                    {m['billing.stripe_publishable_key_help']()}
                  </p>
                </div>
              )}
            />

            <form.Field
              name="secretKey"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="stripe-secret-key">{m['billing.stripe_secret_key']()}</Label>
                  <Input
                    id="stripe-secret-key"
                    type="password"
                    value={field.state.value}
                    onChange={(e) => {
                      field.handleChange(e.target.value)
                      setHasChanges(true)
                    }}
                    placeholder="sk_test_..."
                    data-testid="stripe-secret-key-input"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-red-500">
                        {getFieldErrorMessage(field.state.meta.errors[0])}
                      </p>
                    )}
                  <p className="text-xs text-muted-foreground">
                    {m['billing.stripe_secret_key_help']()}
                  </p>
                </div>
              )}
            />

            <form.Field
              name="webhookSecret"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="stripe-webhook-secret">
                    {m['billing.stripe_webhook_secret']()}
                  </Label>
                  <Input
                    id="stripe-webhook-secret"
                    type="password"
                    value={field.state.value || ''}
                    onChange={(e) => {
                      field.handleChange(e.target.value)
                      setHasChanges(true)
                    }}
                    placeholder="whsec_..."
                    data-testid="stripe-webhook-secret-input"
                  />
                  {(field.state.meta.isTouched || form.state.isSubmitted) &&
                    field.state.meta.errors.length > 0 && (
                      <p className="text-sm text-red-500">
                        {getFieldErrorMessage(field.state.meta.errors[0])}
                      </p>
                    )}
                  <p className="text-xs text-muted-foreground">
                    {m['billing.stripe_webhook_secret_help']()}
                  </p>
                </div>
              )}
            />

            <DialogFooter>
              <form.Subscribe
                selector={(state) => ({
                  canSubmit: state.canSubmit,
                  isSubmitting: state.isSubmitting,
                })}
                children={({ canSubmit, isSubmitting }) => (
                  <>
                    <Button
                      type="button"
                      variant="outline"
                      onClick={() => onOpenChange(false)}
                      disabled={isSubmitting}
                    >
                      {m['common.cancel']()}
                    </Button>
                    <Button
                      type="submit"
                      disabled={isSubmitting || !canSubmit}
                      data-testid="stripe-save-button"
                    >
                      {isSubmitting ? m['shared.saving']() : m['common.save']()}
                    </Button>
                  </>
                )}
              />
            </DialogFooter>
          </form>
        </AppForm>
      </DialogContent>
    </Dialog>
  )
}

interface StripeConfigFormPageProps {
  realmId: string
  mode: 'create' | 'edit'
  initialValues?: Partial<StripeConfigFormValues>
}

export function StripeConfigFormPage({ realmId, mode, initialValues }: StripeConfigFormPageProps) {
  const navigate = useNavigate()
  const isEditing = mode === 'edit'

  const defaultValues = useMemo(() => getStripeConfigDefaults(initialValues), [initialValues])

  const saveMutation = useSaveConfigMutation<StripeConfigFormValues>({
    realmId,
    providerName: 'Stripe',
    isEditing,
    mutationFn: async (data) => {
      const response = await batchUpsertRealmConfigs({
        path: { realmId },
        body: { configs: buildStripeConfigRequest(data) },
      })
      if (response.error) throw response.error
    },
  })

  const form = useAppForm({
    schema: stripeConfigSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      if (
        !requireFieldOnCreate(
          form,
          isEditing,
          'publishableKey',
          value.publishableKey,
          m['billing.stripe_publishable_key_required']()
        )
      )
        return
      if (
        !requireFieldOnCreate(
          form,
          isEditing,
          'secretKey',
          value.secretKey,
          m['billing.stripe_secret_key_required']()
        )
      )
        return
      await saveMutation.mutateAsync(value)
    },
  })

  useEffect(() => {
    form.reset(defaultValues)
  }, [defaultValues, form])

  const handleCancel = () => {
    navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
  }

  const isSubmitting = saveMutation.isPending

  return (
    <div className="container max-w-3xl mx-auto py-6 px-6" data-testid="stripe-config-form-page">
      <PageHeader
        title={isEditing ? m['billing.edit_stripe']() : m['billing.configure_stripe']()}
        headingTestId="stripe-config-form-page-heading"
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
        data-testid="stripe-config-page-form"
        className="space-y-6 pt-6"
      >
        <AppForm>
          <div className="space-y-6">
            <SwitchField
              form={form}
              name="enabled"
              label={m['billing.stripe_enable']()}
              description={m['billing.stripe_enable_description']()}
              dataTestId="page-stripe-enabled-switch"
            />

            <PasswordField
              form={form}
              name="publishableKey"
              label={m['billing.stripe_publishable_key']()}
              dataTestId="page-stripe-publishable-key-input"
              placeholder="pk_test_..."
              helpText={isEditing ? m['billing.leave_empty_keep']() : undefined}
              required={!isEditing}
            />

            <PasswordField
              form={form}
              name="secretKey"
              label={m['billing.stripe_secret_key']()}
              dataTestId="page-stripe-secret-key-input"
              placeholder="sk_test_..."
              helpText={isEditing ? m['billing.leave_empty_keep']() : undefined}
              required={!isEditing}
            />

            <PasswordField
              form={form}
              name="webhookSecret"
              label={m['billing.stripe_webhook_secret']()}
              dataTestId="page-stripe-webhook-secret-input"
              placeholder="whsec_..."
              helpText={m['billing.stripe_webhook_secret_help']()}
            />
          </div>
        </AppForm>

        <FormActionBar
          onCancel={handleCancel}
          isSubmitting={isSubmitting}
          isEditing={isEditing}
          cancelTestId="stripe-config-page-cancel-button"
          submitTestId="stripe-config-page-submit-button"
        />
      </form>
    </div>
  )
}
