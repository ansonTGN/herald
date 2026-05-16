import { createFileRoute } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { listRealmConfigs, batchUpsertRealmConfigs, updateRealm } from '@/lib/api-generated'
import type { UpsertRealmConfigRequest } from '@/lib/api-generated/types.gen'
import { TOTPConfigForm as TOTPConfigFormComponent } from '@/components/realm-config/totp-config-form'
import { RegistrationConfigForm as RegistrationConfigFormComponent } from '@/components/realm-config/registration-config-form'
import { ProviderConfigPage } from '@/components/oauth-config/provider-config-page'
import { useAuth } from '@/hooks/use-auth'
import { toast } from 'sonner'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import type { TOTPConfigForm, RegistrationConfigForm } from '@/lib/schemas/realm-config'
import {
  parseTOTPConfig,
  parseRegistrationConfig,
  buildTOTPConfigRequest,
  buildRegistrationConfigRequest,
} from '@/lib/realm-config-utils'
import { useState, useEffect } from 'react'
import { PageHeader } from '@/components/shared'
import { queryKeys, realmQueryOptions } from '@/data/query-options'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { updateRealmSchema, type UpdateRealmFormData } from '@/lib/schemas/realm'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { TextField } from '@/components/shared/form-fields/text-field'
import { TextareaField } from '@/components/shared/form-fields/textarea-field'

export const Route = createFileRoute('/$realmId/manage/settings')({
  component: SettingsPage,
})

function GeneralTab({ realmId }: { realmId: string }) {
  const { data: realm, isLoading } = useQuery(realmQueryOptions(realmId))
  const auth = useAuth()
  const canUpdate = auth.permissions?.includes('settings.manage') ?? true

  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (data: UpdateRealmFormData) => updateRealm({ path: { realmId }, body: data }),
    getSuccessMessage: () => 'Realm updated successfully',
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

  if (isLoading) return <div>Loading...</div>

  return (
    <Card>
      <CardContent className="space-y-4 max-w-lg pt-6">
        <div className="space-y-2">
          <Label>Realm ID</Label>
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
              label="Realm Name"
              inputId="general-realm-name"
              dataTestId="general-realm-name-input"
              disabled={!canUpdate}
            />
            <div className="mt-4">
              <TextareaField
                form={form}
                name="description"
                label="Description"
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
                  {isSubmitting ? 'Saving...' : 'Save'}
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
  const canViewConfig = auth.permissions?.includes('settings.view') ?? true // Temporary default
  const canUpdateConfig = auth.permissions?.includes('settings.manage') ?? true // Temporary default

  // Get realm configuration list
  const {
    data: configs = [],
    isLoading,
    error,
  } = useQuery({
    queryKey: queryKeys.realmConfig(realmId),
    queryFn: async () => {
      const response = await listRealmConfigs({ path: { realmId } })
      if (response.error) {
        throw response.error
      }
      return response.data
    },
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
      queryClient.invalidateQueries({ queryKey: queryKeys.realmConfig(realmId) })
      toast.success('Configuration saved successfully')
    },
    onError: (error: unknown) => {
      console.error('Failed to save config:', error)

      let errorMessage = 'Failed to save configuration'

      if (error instanceof Error) {
        errorMessage = error.message
      } else if (typeof error === 'object' && error !== null) {
        const err = error as {
          response?: { status?: number; data?: { message?: string } }
        }
        const statusCode = err.response?.status

        if (statusCode === 401) {
          errorMessage = 'Unauthorized: Please log in again'
        } else if (statusCode === 403) {
          errorMessage = 'Forbidden: You do not have permission to modify this configuration'
        } else if (statusCode === 500) {
          errorMessage = 'Server error: Please try again later'
        } else if (err.response?.data?.message) {
          errorMessage = err.response.data.message
        }
      }

      toast.error(errorMessage)
    },
  })

  // Permission check after hooks
  if (!canViewConfig) {
    return (
      <div className="space-y-6">
        <div className="text-destructive">
          Access denied: You do not have permission to view realm configuration
        </div>
      </div>
    )
  }

  // Handle loading and error states
  if (isLoading) {
    return <div>Loading...</div>
  }

  if (error) {
    const errorMessage = error instanceof Error ? error.message : 'Unknown error'
    toast.error(`Failed to load configuration: ${errorMessage}`)
    return <div>Error loading configuration</div>
  }

  // Parse configuration data
  const totpConfig = parseTOTPConfig(configs || [])
  const registrationConfig = parseRegistrationConfig(configs || [])

  // Save TOTP configuration
  async function saveTOTPConfig(config: TOTPConfigForm) {
    if (!canUpdateConfig) {
      toast.error('Access denied: You do not have permission to modify configuration')
      return
    }

    await mutation.mutateAsync([buildTOTPConfigRequest(config)])
  }

  // Save Registration configuration
  async function saveRegistrationConfig(config: RegistrationConfigForm) {
    if (!canUpdateConfig) {
      toast.error('Access denied: You do not have permission to modify configuration')
      return
    }

    await mutation.mutateAsync(buildRegistrationConfigRequest(config))
  }

  return (
    <div className="space-y-6" data-testid="settings-page">
      <PageHeader title="Settings" />

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
          <TabsTrigger value="general" data-testid="general-tab">
            General
          </TabsTrigger>
          <TabsTrigger value="totp" data-testid="totp-tab">
            TOTP
          </TabsTrigger>
          <TabsTrigger value="registration" data-testid="registration-tab">
            Registration
          </TabsTrigger>
          <TabsTrigger value="providers" data-testid="providers-tab">
            Providers
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

        <TabsContent value="registration">
          <RegistrationConfigFormComponent
            realmId={realmId}
            initialConfig={registrationConfig}
            onSave={saveRegistrationConfig}
            isLoading={isLoading}
            disabled={!canUpdateConfig}
          />
        </TabsContent>

        <TabsContent value="providers">
          <ProviderConfigPage realmId={realmId} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
