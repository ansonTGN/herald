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
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { queryKeys } from '@/data/query-options'
import { ArrowLeft } from 'lucide-react'
import { TextField, SwitchField } from '@/components/shared/form-fields'

type MutationResult = CreateApiKeyResponse | ApiKeyListItem

interface ApiKeyFormPageProps {
  mode: 'create' | 'edit'
  realmId: string
  apiKey?: ApiKeyListItem
}

export function ApiKeyFormPage({ mode, realmId, apiKey }: ApiKeyFormPageProps) {
  const isCreate = mode === 'create'
  const navigate = useNavigate()

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
    onSuccess: (data) => {
      if (isCreate) {
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
      await mutate(value)
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
