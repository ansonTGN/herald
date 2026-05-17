import { useEffect, useMemo, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
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
        if (
          !open ||
          !hasChanges ||
          confirm('You have unsaved changes. Are you sure you want to close?')
        ) {
          onOpenChange(open)
          if (!open) setHasChanges(false)
        }
      }}
    >
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            {mode === 'create' ? 'Configure Stripe' : 'Edit Stripe Configuration'}
          </DialogTitle>
          <DialogDescription>
            Configure Stripe as your payment provider for subscriptions and one-time payments
          </DialogDescription>
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
                    <Label htmlFor="stripe-enabled">Enable Stripe</Label>
                    <p className="text-sm text-muted-foreground">Allow users to pay with Stripe</p>
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
                  <Label htmlFor="stripe-publishable-key">Publishable Key</Label>
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
                    Starts with <code>pk_</code>. Found in Stripe Dashboard → Developers → API keys
                  </p>
                </div>
              )}
            />

            <form.Field
              name="secretKey"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="stripe-secret-key">Secret Key</Label>
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
                    Starts with <code>sk_</code>. Found in Stripe Dashboard → Developers → API keys
                  </p>
                </div>
              )}
            />

            <form.Field
              name="webhookSecret"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="stripe-webhook-secret">Webhook Secret</Label>
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
                    Optional. Starts with <code>whsec_</code>. Configure in Stripe Dashboard →
                    Developers → Webhooks
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
                      Cancel
                    </Button>
                    <Button
                      type="submit"
                      disabled={isSubmitting || !canSubmit}
                      data-testid="stripe-save-button"
                    >
                      {isSubmitting ? 'Saving...' : 'Save'}
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

export function StripeConfigFormPage({
  realmId,
  mode,
  initialValues,
}: StripeConfigFormPageProps) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const isEditing = mode === 'edit'

  const defaultValues = useMemo(() => getStripeConfigDefaults(initialValues), [initialValues])

  const form = useAppForm({
    schema: stripeConfigSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      await saveMutation.mutateAsync(value)
    },
  })

  useEffect(() => {
    form.reset(defaultValues)
  }, [defaultValues, form])

  const saveMutation = useMutation({
    mutationFn: async (data: StripeConfigFormValues) => {
      const response = await batchUpsertRealmConfigs({
        path: { realmId },
        body: { configs: [buildStripeConfigRequest(data)] },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success(
        isEditing ? 'Stripe configuration updated successfully' : 'Stripe configuration created successfully'
      )
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['payment-providers', realmId] }),
        queryClient.invalidateQueries({ queryKey: ['realmConfig', realmId] }),
      ])
      navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
    },
    onError: (error: { status?: number; message?: string }) => {
      toast.error(`Failed to save configuration: ${error?.message || 'Unknown error'}`)
    },
  })

  const handleCancel = () => {
    navigate({ to: '/$realmId/manage/billing/payment-providers', params: { realmId } })
  }

  const isSubmitting = saveMutation.isPending

  return (
    <div className="container max-w-3xl mx-auto py-6 px-6" data-testid="stripe-config-form-page">
      <PageHeader
        title={isEditing ? 'Edit Stripe Configuration' : 'Configure Stripe'}
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
              label="Enable Stripe"
              description="Allow users to pay with Stripe"
              dataTestId="page-stripe-enabled-switch"
            />

            <PasswordField
              form={form}
              name="publishableKey"
              label="Publishable Key"
              dataTestId="page-stripe-publishable-key-input"
              placeholder="pk_test_..."
              helpText={<>Starts with <code>pk_</code>. Found in Stripe Dashboard → Developers → API keys</>}
              required
            />

            <PasswordField
              form={form}
              name="secretKey"
              label="Secret Key"
              dataTestId="page-stripe-secret-key-input"
              placeholder="sk_test_..."
              helpText={<>Starts with <code>sk_</code>. Found in Stripe Dashboard → Developers → API keys</>}
              required
            />

            <PasswordField
              form={form}
              name="webhookSecret"
              label="Webhook Secret"
              dataTestId="page-stripe-webhook-secret-input"
              placeholder="whsec_..."
              helpText={<>Optional. Starts with <code>whsec_</code>. Configure in Stripe Dashboard → Developers → Webhooks</>}
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
