import { useState } from 'react'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  createApiKeySchema,
  updateApiKeySchema,
  type CreateApiKeyFormData,
  type UpdateApiKeyFormData,
} from '@/lib/schemas/api-key-forms'
import { createApiKey, updateApiKey } from '@/lib/api-generated'
import type { CreateApiKeyResponse, ApiKeyListItem } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { usePermission } from '@/hooks/use-permission'
import { PERMISSION } from '@/lib/constants/auth-constants'
import { useNavigate } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  clientAppsQueryOptions,
  queryKeys,
  rolesQueryOptions,
  updateApiKeyRolesMutation,
} from '@/data/query-options'
import { toast } from 'sonner'
import { ArrowLeft } from 'lucide-react'
import { TextField, SwitchField } from '@/components/shared/form-fields'
import { RoleSelector } from '@/components/shared/role-selector'
import { ClientAppSelector } from '@/components/shared/client-app-selector'

type MutationResult = CreateApiKeyResponse | ApiKeyListItem

interface ApiKeyFormPageProps {
  mode: 'create' | 'edit'
  realmId: string
  apiKey?: ApiKeyListItem
}

export function ApiKeyFormPage({ mode, realmId, apiKey }: ApiKeyFormPageProps) {
  const isCreate = mode === 'create'
  const navigate = useNavigate()
  const { hasPermission } = usePermission()
  const canManageRoles = hasPermission(PERMISSION.ROLES_MANAGE)
  const [selectedRoleIds, setSelectedRoleIds] = useState<string[]>([])

  const { data: rolesData } = useQuery({
    ...rolesQueryOptions(realmId),
    enabled: isCreate && canManageRoles,
  })

  const { data: clientAppsData } = useQuery({
    ...clientAppsQueryOptions(realmId, { page: 0, pageSize: 100 }),
    enabled: isCreate,
  })

  const goToList = () => navigate({ to: '/$realmId/manage/api-keys', params: { realmId } })

  const { isSubmitting, mutate } = useFormMutation<
    MutationResult,
    CreateApiKeyFormData | UpdateApiKeyFormData
  >({
    mutationFn: (data) => {
      if (isCreate) {
        return createApiKey({
          path: { realmId },
          body: data as CreateApiKeyFormData,
        }).then((response) => {
          if (response.error) throw response.error
          return response.data as CreateApiKeyResponse
        })
      }
      return updateApiKey({
        path: { realmId, apiKeyId: apiKey!.id },
        body: data as UpdateApiKeyFormData,
      }).then((response) => {
        if (response.error) throw response.error
        return response.data as ApiKeyListItem
      })
    },
    invalidateQueries: [queryKeys.apiKeysList(realmId)],
    getSuccessMessage: () =>
      isCreate ? 'API Key created successfully' : 'API Key updated successfully',
    onSuccess: async (data) => {
      if (isCreate) {
        if (selectedRoleIds.length > 0 && canManageRoles) {
          try {
            await updateApiKeyRolesMutation(realmId, data.id, selectedRoleIds)
          } catch {
            toast.error(
              'API Key created, but role binding failed. You can manage roles from the list page later.'
            )
          }
        }
        void navigate({
          to: '/$realmId/manage/api-keys/reveal',
          params: { realmId },
          state: { keyData: data } as unknown as Parameters<typeof navigate>[0]['state'],
        })
      } else {
        void goToList()
      }
    },
  })

  const form = useAppForm({
    schema: isCreate ? createApiKeySchema : updateApiKeySchema,
    defaultValues: isCreate
      ? ({
          name: '',
          expiresAt: '',
        } as CreateApiKeyFormData)
      : ({
          name: apiKey?.name ?? '',
          enabled: apiKey?.enabled ?? true,
          expiresAt: apiKey?.expiresAt ?? null,
        } as UpdateApiKeyFormData),
    onSubmit: async ({ value }) => {
      // Parse through schema to apply transforms (e.g. empty string -> undefined,
      // datetime-local -> RFC 3339)
      const schema = isCreate ? createApiKeySchema : updateApiKeySchema
      const parsed = schema.parse(value)
      await mutate(parsed)
    },
  })

  return (
    <div className="space-y-6" data-testid="api-key-form-page">
      <div className="flex items-center gap-4">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => goToList()}
          data-testid="api-key-form-back-button"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div>
          <h1 className="text-2xl font-bold" data-testid="page-title">
            {isCreate ? 'Create API Key' : 'Edit API Key'}
          </h1>
          <p className="text-muted-foreground text-sm">
            {isCreate
              ? 'Create a new API key for programmatic access.'
              : 'Update the API key configuration.'}
          </p>
        </div>
      </div>

      <AppForm>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            e.stopPropagation()
            form.handleSubmit()
          }}
          className="max-w-lg space-y-6"
        >
          <TextField
            form={form}
            name="name"
            label="Name"
            inputId="api-key-name"
            dataTestId="api-key-name-input"
            placeholder="My API Key"
            required
          />

          {isCreate && (
            <form.Field
              name="clientAppId"
              children={(field) => (
                <div className="space-y-2">
                  <Label>Client App</Label>
                  <ClientAppSelector
                    clientApps={(clientAppsData?.items ?? []).map((app) => ({
                      id: app.id,
                      name: app.name,
                      clientId: app.clientId,
                    }))}
                    value={field.state.value}
                    onChange={field.handleChange}
                    disabled={isSubmitting}
                  />
                </div>
              )}
            />
          )}

          {!isCreate && (
            <SwitchField
              form={form}
              name="enabled"
              label="Enabled"
              inputId="api-key-enabled"
              dataTestId="api-key-enabled-switch"
            />
          )}

          <form.Field
            name="expiresAt"
            children={(field) => (
              <div className="space-y-2">
                <Label htmlFor="api-key-expires-at">Expires At</Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="api-key-expires-at"
                    type="datetime-local"
                    value={field.state.value ?? ''}
                    onChange={(e) =>
                      field.handleChange(
                        isCreate
                          ? (e.target.value as string | undefined)
                          : (e.target.value as string | null) || null
                      )
                    }
                    data-testid="api-key-expires-at-input"
                  />
                  {!isCreate && field.state.value && (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => field.handleChange(null)}
                      data-testid="api-key-expires-at-clear-button"
                    >
                      Clear
                    </Button>
                  )}
                </div>
              </div>
            )}
          />

          {isCreate && canManageRoles && (
            <div className="space-y-2">
              <Label>Roles (Optional)</Label>
              <RoleSelector
                roles={(rolesData ?? []).map((r) => ({ id: r.id, name: r.name }))}
                selectedRoleIds={selectedRoleIds}
                onChange={setSelectedRoleIds}
                disabled={isSubmitting}
                placeholder="Select roles to assign after creation"
              />
              <p className="text-xs text-muted-foreground">
                Selected roles will be assigned to the API key after creation.
              </p>
            </div>
          )}

          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => goToList()}
              data-testid="cancel-button"
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting} data-testid="submit-button">
              {isSubmitting
                ? isCreate
                  ? 'Creating...'
                  : 'Saving...'
                : isCreate
                  ? 'Create'
                  : 'Save Changes'}
            </Button>
          </div>
        </form>
      </AppForm>
    </div>
  )
}
