import React from 'react'
import { useMutation } from '@tanstack/react-query'
import { useStore } from '@tanstack/react-form'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  emailConfigSchema,
  type EmailConfigForm as EmailConfigFormValues,
} from '@/lib/schemas/realm-config'
import type { EmailStatusResponse } from '@/lib/api-generated/types.gen'
import { emailTest } from '@/lib/api-generated/sdk.gen'
import { handleApiResponse } from '@/lib/api-utils'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group'
import { Label } from '@/components/ui/label'
import { TextField } from '@/components/shared/form-fields/text-field'
import { PasswordField } from '@/components/shared/form-fields/password-field'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Input } from '@/components/ui/input'
import { getErrorMessage } from '@/lib/error-utils'
import { useFormSubmit } from './use-form-submit'

interface EmailConfigFormProps {
  realmId: string
  initialConfig?: EmailConfigFormValues
  onSave: (config: EmailConfigFormValues) => Promise<void>
  isLoading?: boolean
  disabled?: boolean
  emailStatus?: EmailStatusResponse | null
  emailStatusError?: string | null
}

export function EmailConfigForm({
  realmId,
  initialConfig,
  onSave,
  isLoading,
  disabled,
  emailStatus,
  emailStatusError,
}: EmailConfigFormProps) {
  const [saveError, setSaveError] = React.useState<string | null>(null)
  const [testRecipient, setTestRecipient] = React.useState('')

  const { handleSubmit, isSubmitting } = useFormSubmit(onSave, disabled)

  const form = useAppForm({
    schema: emailConfigSchema,
    defaultValues: initialConfig || {
      provider: 'resend' as const,
      fromAddress: '',
      smtpPort: '587',
      smtpEncryption: 'starttls' as const,
    },
    onSubmit: async ({ value }) => {
      setSaveError(null)
      try {
        await handleSubmit(value)
      } catch (error) {
        setSaveError(getErrorMessage(error))
      }
    },
  })

  const provider = useStore(form.store, (state) => state.values.provider)

  const testEmailMutation = useMutation({
    mutationFn: async (recipient: string) => {
      const response = await emailTest({
        path: { realmId },
        body: { recipient },
      })
      return handleApiResponse(response)
    },
  })

  const handleSendTestEmail = () => {
    if (!testRecipient) return
    testEmailMutation.mutate(testRecipient)
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <CardTitle>Email Configuration</CardTitle>
            <CardDescription>Configure email delivery for this realm</CardDescription>
          </div>
          {emailStatus && (
            <Badge
              variant={emailStatus.configured ? 'default' : 'outline'}
              className={
                emailStatus.configured
                  ? 'bg-green-100 text-green-800 border-green-300'
                  : 'bg-amber-100 text-amber-800 border-amber-300'
              }
              data-testid="email-config-status-badge"
            >
              {emailStatus.configured ? 'Email is configured' : 'Email is not configured'}
            </Badge>
          )}
        </div>
      </CardHeader>
      {emailStatusError && (
        <div className="px-6 pb-2">
          <p className="text-sm text-destructive" data-testid="email-status-error">
            {emailStatusError}
          </p>
        </div>
      )}
      <CardContent>
        <AppForm>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              setSaveError(null)
              form.handleSubmit()
            }}
            className="space-y-6"
          >
            <form.Field
              name="provider"
              children={(field) => (
                <div className="space-y-2">
                  <Label>Email Provider</Label>
                  <RadioGroup
                    value={field.state.value}
                    onValueChange={(value) => field.handleChange(value as 'resend' | 'smtp')}
                    disabled={disabled}
                    className="flex gap-4"
                  >
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem
                        value="resend"
                        id="provider-resend"
                        data-testid="email-provider-resend"
                      />
                      <Label htmlFor="provider-resend">Resend API</Label>
                    </div>
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem
                        value="smtp"
                        id="provider-smtp"
                        data-testid="email-provider-smtp"
                      />
                      <Label htmlFor="provider-smtp">SMTP</Label>
                    </div>
                  </RadioGroup>
                </div>
              )}
            />

            <TextField
              form={form}
              name="fromAddress"
              label="From Address"
              inputId="email-from-address"
              dataTestId="email-from-address-input"
              type="email"
              placeholder="noreply@example.com"
              disabled={disabled}
              required
            />

            {provider === 'resend' && (
              <PasswordField
                form={form}
                name="resendApiKey"
                label="API Key"
                inputId="email-resend-api-key"
                dataTestId="email-resend-api-key-input"
                placeholder="re_xxxxxxxx"
                disabled={disabled}
                helpText="Enter a new API key to replace the existing one"
              />
            )}

            {provider === 'smtp' && (
              <div className="space-y-4">
                <TextField
                  form={form}
                  name="smtpHost"
                  label="SMTP Host"
                  inputId="email-smtp-host"
                  dataTestId="email-smtp-host-input"
                  placeholder="smtp.example.com"
                  disabled={disabled}
                />

                <TextField
                  form={form}
                  name="smtpPort"
                  label="Port"
                  inputId="email-smtp-port"
                  dataTestId="email-smtp-port-input"
                  placeholder="587"
                  disabled={disabled}
                />

                <form.Field
                  name="smtpEncryption"
                  children={(field) => (
                    <div className="space-y-2">
                      <Label htmlFor="email-smtp-encryption">Encryption</Label>
                      <Select
                        value={field.state.value}
                        onValueChange={(value) => field.handleChange(value as 'starttls' | 'ssl')}
                        disabled={disabled}
                      >
                        <SelectTrigger
                          id="email-smtp-encryption"
                          data-testid="email-smtp-encryption-select"
                        >
                          <SelectValue placeholder="Select encryption" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="starttls">STARTTLS</SelectItem>
                          <SelectItem value="ssl">SSL</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  )}
                />

                <TextField
                  form={form}
                  name="smtpUsername"
                  label="Username"
                  inputId="email-smtp-username"
                  dataTestId="email-smtp-username-input"
                  placeholder="user@example.com"
                  disabled={disabled}
                />

                <PasswordField
                  form={form}
                  name="smtpPassword"
                  label="Password"
                  inputId="email-smtp-password"
                  dataTestId="email-smtp-password-input"
                  placeholder="Enter password"
                  disabled={disabled}
                />
              </div>
            )}

            {saveError && (
              <p className="text-sm text-destructive" data-testid="email-save-error">
                {saveError}
              </p>
            )}

            <div className="flex items-center justify-between gap-4">
              <div className="flex items-center gap-2">
                <Input
                  placeholder="Recipient email"
                  value={testRecipient}
                  onChange={(e) => setTestRecipient(e.target.value)}
                  disabled={disabled}
                  type="email"
                  className="max-w-[260px]"
                  data-testid="email-test-recipient-input"
                />
                <Button
                  type="button"
                  variant="outline"
                  onClick={handleSendTestEmail}
                  disabled={disabled || !testRecipient || testEmailMutation.isPending}
                  data-testid="email-test-button"
                >
                  {testEmailMutation.isPending ? 'Sending...' : 'Send Test Email'}
                </Button>
              </div>

              <Button
                type="submit"
                disabled={isLoading || isSubmitting || disabled}
                data-testid="email-save-button"
              >
                {isSubmitting ? 'Saving...' : 'Save'}
              </Button>
            </div>

            {testEmailMutation.isError && (
              <p className="text-sm text-destructive" data-testid="email-test-error">
                {getErrorMessage(testEmailMutation.error)}
              </p>
            )}

            {testEmailMutation.isSuccess && (
              <p className="text-sm text-green-600" data-testid="email-test-success">
                {testEmailMutation.data?.message || 'Test email sent successfully'}
              </p>
            )}
          </form>
        </AppForm>
      </CardContent>
    </Card>
  )
}
