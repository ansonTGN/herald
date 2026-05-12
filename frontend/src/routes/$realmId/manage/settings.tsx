import { createFileRoute } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { listRealmConfigs, batchUpsertRealmConfigs } from '@/lib/api-generated'
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
import { useState } from 'react'
import { PageHeader } from '@/components/shared'
import { queryKeys } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/manage/settings')({
  component: SettingsPage,
})

function SettingsPage() {
  const { realmId } = Route.useParams()
  const queryClient = useQueryClient()
  const auth = useAuth()
  const [activeTab, setActiveTab] = useState('totp')

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
      <div className="p-6">
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
    <div className="p-6" data-testid="settings-page">
      <PageHeader title="Settings" description="Manage realm configuration" className="mb-6" />

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
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
