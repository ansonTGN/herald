import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  stripeConfigSchema,
  type StripeConfigForm as StripeConfigFormValues,
  getStripeConfigDefaults,
} from '@/lib/schemas/stripe-config'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { getFieldErrorMessage } from '@/lib/error-utils'

interface StripeConfigFormProps {
  realmId: string
  initialConfig?: StripeConfigFormValues
  onSave: (config: StripeConfigFormValues) => Promise<void>
  isLoading?: boolean
  disabled?: boolean
}

export function StripeConfigForm({
  realmId: _realmId,
  initialConfig,
  onSave,
  isLoading,
  disabled,
}: StripeConfigFormProps) {
  const form = useAppForm({
    schema: stripeConfigSchema,
    defaultValues: initialConfig || getStripeConfigDefaults(),
    onSubmit: async ({ value }) => {
      if (disabled) {
        throw new Error(
          'Form is disabled. You do not have permission to modify this configuration.'
        )
      }
      try {
        await onSave(value)
      } catch (error) {
        console.error('Failed to save Stripe configuration:', error)
      }
    },
  })

  return (
    <Card>
      <CardHeader>
        <CardTitle>Stripe Configuration</CardTitle>
        <CardDescription>
          Configure Stripe as your payment provider for subscriptions and one-time payments
        </CardDescription>
      </CardHeader>
      <CardContent>
        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              form.handleSubmit()
            }}
            className="space-y-4"
          >
            {/* Enable Stripe */}
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
                    onCheckedChange={(checked) => field.handleChange(checked)}
                    disabled={disabled}
                    data-testid="stripe-enabled-switch"
                  />
                </div>
              )}
            />

            {/* Publishable Key */}
            <form.Field
              name="publishableKey"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="stripe-publishable-key">Publishable Key</Label>
                  <Input
                    id="stripe-publishable-key"
                    type="password"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="pk_test_..."
                    disabled={disabled}
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

            {/* Secret Key */}
            <form.Field
              name="secretKey"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="stripe-secret-key">Secret Key</Label>
                  <Input
                    id="stripe-secret-key"
                    type="password"
                    value={field.state.value}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="sk_test_..."
                    disabled={disabled}
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

            {/* Webhook Secret */}
            <form.Field
              name="webhookSecret"
              children={(field) => (
                <div className="space-y-2">
                  <Label htmlFor="stripe-webhook-secret">Webhook Secret</Label>
                  <Input
                    id="stripe-webhook-secret"
                    type="password"
                    value={field.state.value || ''}
                    onChange={(e) => field.handleChange(e.target.value)}
                    placeholder="whsec_..."
                    disabled={disabled}
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

            <div className="flex justify-end">
              <form.Subscribe
                selector={(state) => ({
                  canSubmit: state.canSubmit,
                  isSubmitting: state.isSubmitting,
                })}
                children={({ canSubmit, isSubmitting }) => (
                  <Button
                    type="submit"
                    disabled={isLoading || isSubmitting || disabled || !canSubmit}
                    data-testid="stripe-save-button"
                  >
                    {isSubmitting ? 'Saving...' : 'Save'}
                  </Button>
                )}
              />
            </div>
          </form>
        </AppForm>
      </CardContent>
    </Card>
  )
}
