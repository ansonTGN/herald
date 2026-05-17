import { useEffect, useMemo } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  creemConfigSchema,
  type CreemConfigForm as CreemConfigFormValues,
  getCreemConfigDefaults,
} from '@/lib/schemas/creem-config'
import { PageHeader } from '@/components/shared/page-header'
import { FormActionBar } from '@/components/shared/form-action-bar'
import { SwitchField, PasswordField, NumberField } from '@/components/shared/form-fields'
import { batchUpsertRealmConfigs } from '@/lib/api-generated/sdk.gen'
import { buildCreemConfigRequest } from '@/lib/creem-config-utils'

interface CreemConfigFormPageProps {
  realmId: string
  mode: 'create' | 'edit'
  initialValues?: Partial<CreemConfigFormValues>
}

export function CreemConfigFormPage({
  realmId,
  mode,
  initialValues,
}: CreemConfigFormPageProps) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const isEditing = mode === 'edit'

  const defaultValues = useMemo(() => getCreemConfigDefaults(initialValues), [initialValues])

  const form = useAppForm({
    schema: creemConfigSchema,
    defaultValues,
    onSubmit: async ({ value }) => {
      await saveMutation.mutateAsync(value)
    },
  })

  useEffect(() => {
    form.reset(defaultValues)
  }, [defaultValues, form])

  const saveMutation = useMutation({
    mutationFn: async (data: CreemConfigFormValues) => {
      const response = await batchUpsertRealmConfigs({
        path: { realmId },
        body: { configs: buildCreemConfigRequest(data) },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async () => {
      toast.success(
        isEditing ? 'Creem configuration updated successfully' : 'Creem configuration created successfully'
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
    <div className="container max-w-3xl mx-auto py-6 px-6" data-testid="creem-config-form-page">
      <PageHeader
        title={isEditing ? 'Edit Creem Configuration' : 'Configure Creem'}
        headingTestId="creem-config-form-page-heading"
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
        data-testid="creem-config-page-form"
        className="space-y-6 pt-6"
      >
        <AppForm>
          <div className="space-y-6">
            <SwitchField
              form={form}
              name="enabled"
              label="Enable Creem"
              description="Allow users to pay with Creem"
              dataTestId="page-creem-enabled-switch"
            />

            <PasswordField
              form={form}
              name="apiKey"
              label="API Key"
              dataTestId="page-creem-api-key-input"
              placeholder="ck_test_..."
              helpText={<>Starts with <code>ck_test_</code> or <code>ck_live_</code></>}
              required
            />

            <NumberField
              form={form}
              name="timeout"
              label="Timeout"
              dataTestId="page-creem-timeout-input"
              min={1}
              max={120}
              helpText="HTTP request timeout in seconds (1-120, default: 30)"
            />

            <PasswordField
              form={form}
              name="webhookSecret"
              label="Webhook Secret"
              dataTestId="page-creem-webhook-secret-input"
              placeholder="whsec_..."
              helpText={<>Optional. Starts with <code>whsec_</code></>}
            />
          </div>
        </AppForm>

        <FormActionBar
          onCancel={handleCancel}
          isSubmitting={isSubmitting}
          isEditing={isEditing}
          cancelTestId="creem-config-page-cancel-button"
          submitTestId="creem-config-page-submit-button"
        />
      </form>
    </div>
  )
}
