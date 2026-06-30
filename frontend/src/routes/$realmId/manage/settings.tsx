import { createFileRoute } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { listRealmConfigs, batchUpsertRealmConfigs, updateRealm } from '@/lib/api-generated'
import type { UpsertRealmConfigRequest } from '@/lib/api-generated/types.gen'
import { TOTPConfigForm as TOTPConfigFormComponent } from '@/components/realm-config/totp-config-form'
import { RegistrationConfigForm as RegistrationConfigFormComponent } from '@/components/realm-config/registration-config-form'
import { EmailConfigForm as EmailConfigFormComponent } from '@/components/realm-config/email-config-form'
import { TurnstileConfigForm as TurnstileConfigFormComponent } from '@/components/realm-config/turnstile-config-form'
import { ProviderConfigPage } from '@/components/oauth-config/provider-config-page'
import { LegalAgreementTab } from '@/components/settings/LegalAgreementTab'
import { useAuth } from '@/hooks/use-auth'
import { PERMISSION } from '@/lib/constants/auth-constants'
import { toast } from 'sonner'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import type {
  TOTPConfigForm,
  RegistrationConfigForm,
  EmailConfigForm,
  TurnstileConfigForm,
} from '@/lib/schemas/realm-config'
import {
  parseTOTPConfig,
  parseRegistrationConfig,
  parseEmailConfig,
  parseTurnstileConfig,
  buildTOTPConfigRequest,
  buildRegistrationConfigRequest,
  buildEmailConfigRequest,
  buildTurnstileConfigRequest,
} from '@/lib/realm-config-utils'
import { useState, useEffect } from 'react'
import { PageHeader, AccessDenied } from '@/components/shared'
import { queryKeys, realmQueryOptions, emailStatusQueryOptions } from '@/data/query-options'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { updateRealmSchema, type UpdateRealmFormData } from '@/lib/schemas/realm'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { TextField } from '@/components/shared/form-fields/text-field'
import { TextareaField } from '@/components/shared/form-fields/textarea-field'
import { m } from '@/paraglide/messages'

export const Route = createFileRoute('/$realmId/manage/settings')({
  component: SettingsPage,
})

function GeneralTab({ realmId }: { realmId: string }) {
  const { data: realm, isLoading } = useQuery(realmQueryOptions(realmId))
  const auth = useAuth()
  const canUpdate = auth.permissions?.includes(PERMISSION.SETTINGS_MANAGE) ?? false

  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (data: UpdateRealmFormData) => updateRealm({ path: { realmId }, body: data }),
    getSuccessMessage: () => m['settings.realm_updated_success'](),
    invalidateQueries: [queryKeys.realm(realmId)],
  })

  const form = useAppForm({
    schema: updateRealmSchema,
    defaultValues: {
      name: '',
      description: '',
    },
    onSubmit: async ({ value }) => {
      await mutate(value)
    },
  })

  useEffect(() => {
    if (realm?.name !== undefined) {
      form.setFieldValue('name', realm.name)
    }
    if (realm !== undefined) {
      form.setFieldValue('description', realm.description ?? '')
    }
  }, [realm, form])

  if (isLoading) return <div>{m['settings.general_loading']()}</div>

  return (
    <Card>
      <CardContent className="space-y-4 max-w-lg pt-6">
        <div className="space-y-2">
          <Label>{m['settings.general_realm_id_label']()}</Label>
          <Input value={realmId} disabled data-testid="general-realm-id" />
        </div>

        <AppForm>
          <form
            id="general-realm-form"
            onSubmit={async (e) => {
              e.preventDefault()
              await form.handleSubmit()
            }}
          >
            <TextField
              form={form}
              name="name"
              label={m['settings.general_realm_name_label']()}
              inputId="general-realm-name"
              dataTestId="general-realm-name-input"
              disabled={!canUpdate}
            />
            <div className="mt-4">
              <TextareaField
                form={form}
                name="description"
                label={m['settings.general_description_label']()}
                inputId="general-realm-description"
                dataTestId="general-realm-description-input"
                disabled={!canUpdate}
                rows={3}
              />
            </div>
            {canUpdate && (
              <div className="mt-4">
                <Button
                  type="submit"
                  form="general-realm-form"
                  disabled={isSubmitting}
                  data-testid="general-realm-save"
                >
                  {isSubmitting ? m['settings.general_saving']() : m['settings.general_save']()}
                </Button>
              </div>
            )}
          </form>
        </AppForm>
      </CardContent>
    </Card>
  )
}

function SettingsPage() {
  const { realmId } = Route.useParams()
  const queryClient = useQueryClient()
  const auth = useAuth()
  const [activeTab, setActiveTab] = useState('general')

  // Permission checks
  const canViewConfig = auth.permissions?.includes(PERMISSION.SETTINGS_VIEW) ?? false
  const canUpdateConfig = auth.permissions?.includes(PERMISSION.SETTINGS_MANAGE) ?? false

  // Get realm configuration list
  const {
    data: configs = [],
    isLoading,
    error,
  } = useQuery({
    queryKey: queryKeys.realmConfigs(realmId),
    queryFn: async () => {
      const response = await listRealmConfigs({ path: { realmId } })
      if (response.error) {
        throw response.error
      }
      return response.data
    },
    enabled: !!realmId && canViewConfig,
  })

  // Get email configuration status
  const { data: emailStatusData, error: emailStatusQueryError } = useQuery({
    ...emailStatusQueryOptions(realmId),
    enabled: !!realmId && canViewConfig,
  })

  // Batch update configuration
  const mutation = useMutation({
    mutationFn: (configs: UpsertRealmConfigRequest[]) =>
      batchUpsertRealmConfigs({
        path: { realmId },
        body: { configs },
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realmConfigs(realmId) })
      toast.success(m['settings.config_saved_success']())
    },
    onError: (error: unknown) => {
      console.error('Failed to save config:', error)

      let errorMessage: string = m['settings.config_save_failed']()

      if (error instanceof Error) {
        errorMessage = error.message
      } else if (typeof error === 'object' && error !== null) {
        const err = error as {
          response?: { status?: number; data?: { message?: string } }
        }
        const statusCode = err.response?.status

        if (statusCode === 401) {
          errorMessage = m['settings.config_save_unauthorized']()
        } else if (statusCode === 403) {
          errorMessage = m['settings.config_save_forbidden']()
        } else if (statusCode === 500) {
          errorMessage = m['settings.config_save_server_error']()
        } else if (err.response?.data?.message) {
          errorMessage = err.response.data.message
        }
      }

      toast.error(errorMessage)
    },
  })

  if (!canViewConfig) {
    return <AccessDenied message={m['settings.config_access_denied']()} />
  }

  // Handle loading and error states
  if (isLoading) {
    return <div>{m['settings.config_loading']()}</div>
  }

  if (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error'
    toast.error(m['settings.config_failed_to_load']({ message: errorMessage }))
    return <div>{m['settings.config_error_loading']()}</div>
  }

  // Parse configuration data
  const totpConfig = parseTOTPConfig(configs || [])
  const turnstileConfig = parseTurnstileConfig(configs || [])
  const registrationConfig = parseRegistrationConfig(configs || [])
  const emailConfig = parseEmailConfig(configs || [])

  // Save TOTP configuration
  async function saveTOTPConfig(config: TOTPConfigForm) {
    if (!canUpdateConfig) {
      toast.error(m['settings.config_modify_denied']())
      return
    }

    // The mutation's `onError` surfaces the failure toast and logs the error.
    // Swallow the rejected promise so it doesn't propagate as an unhandled
    // rejection (per vitest config, rejections are expected to be handled in
    // components). Sibling forms share this latent leak; see FE-T06 handoff.
    await mutation.mutateAsync([buildTOTPConfigRequest(config)]).catch(() => {})
  }

  // Save Turnstile configuration
  async function saveTurnstileConfig(config: TurnstileConfigForm) {
    if (!canUpdateConfig) {
      toast.error(m['settings.config_modify_denied']())
      return
    }

    await mutation.mutateAsync(buildTurnstileConfigRequest(config)).catch(() => {})
  }

  // Save Registration configuration
  async function saveRegistrationConfig(config: RegistrationConfigForm) {
    if (!canUpdateConfig) {
      toast.error(m['settings.config_modify_denied']())
      return
    }

    await mutation.mutateAsync(buildRegistrationConfigRequest(config)).catch(() => {})
  }

  // Save Email configuration
  async function saveEmailConfig(config: EmailConfigForm) {
    if (!canUpdateConfig) {
      toast.error(m['settings.config_modify_denied']())
      return
    }

    await mutation.mutateAsync(buildEmailConfigRequest(config)).catch(() => {})
    queryClient.invalidateQueries({ queryKey: queryKeys.emailStatus(realmId) })
  }

  return (
    <div className="space-y-6" data-testid="settings-page">
      <PageHeader title={m['settings.page_title']()} />

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
          <TabsTrigger value="general" data-testid="general-tab">
            {m['settings.tab_general']()}
          </TabsTrigger>
          <TabsTrigger value="totp" data-testid="totp-tab">
            {m['settings.tab_totp']()}
          </TabsTrigger>
          <TabsTrigger value="turnstile" data-testid="turnstile-tab">
            {m['settings.tab_turnstile']()}
          </TabsTrigger>
          <TabsTrigger value="registration" data-testid="registration-tab">
            {m['settings.tab_registration']()}
          </TabsTrigger>
          <TabsTrigger value="email" data-testid="email-tab">
            {m['settings.tab_email']()}
          </TabsTrigger>
          <TabsTrigger value="providers" data-testid="providers-tab">
            {m['settings.tab_providers']()}
          </TabsTrigger>
          <TabsTrigger value="legal" data-testid="legal-tab">
            {m['settings.legal.tab_legal']()}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="general">
          <GeneralTab realmId={realmId} />
        </TabsContent>

        <TabsContent value="totp">
          <TOTPConfigFormComponent
            realmId={realmId}
            initialConfig={totpConfig}
            onSave={saveTOTPConfig}
            isLoading={isLoading}
            disabled={!canUpdateConfig}
          />
        </TabsContent>

        <TabsContent value="turnstile">
          <TurnstileConfigFormComponent
            realmId={realmId}
            initialConfig={turnstileConfig}
            onSave={saveTurnstileConfig}
            isLoading={isLoading}
            disabled={!canUpdateConfig}
          />
        </TabsContent>

        <TabsContent value="registration">
          <RegistrationConfigFormComponent
            realmId={realmId}
            initialConfig={registrationConfig}
            onSave={saveRegistrationConfig}
            isLoading={isLoading}
            disabled={!canUpdateConfig}
            emailConfigured={emailStatusData?.configured ?? false}
          />
        </TabsContent>

        <TabsContent value="email">
          <EmailConfigFormComponent
            realmId={realmId}
            initialConfig={emailConfig}
            onSave={saveEmailConfig}
            isLoading={isLoading}
            disabled={!canUpdateConfig}
            emailStatus={emailStatusData ?? null}
            emailStatusError={
              emailStatusQueryError instanceof Error ? emailStatusQueryError.message : null
            }
          />
        </TabsContent>

        <TabsContent value="providers">
          <ProviderConfigPage realmId={realmId} />
        </TabsContent>

        <TabsContent value="legal">
          <LegalAgreementTab realmId={realmId} canManage={canUpdateConfig} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
