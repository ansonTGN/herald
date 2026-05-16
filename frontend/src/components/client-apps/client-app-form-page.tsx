import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  createClientAppSchema,
  updateClientAppSchema,
  type CreateClientAppFormData,
  type UpdateClientAppFormData,
} from '@/lib/schemas/client-app-forms'
import { createClientApp, updateClientApp } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { useNavigate } from '@tanstack/react-router'
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { RedirectUrisInput, type UriItem } from '@/components/client-apps/redirect-uris-input'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { queryKeys } from '@/data/query-options'
import { toast } from 'sonner'
import { ArrowLeft } from 'lucide-react'
import type { ClientAppItem } from '@/lib/api-generated'

function transformToUriItems(uris: string[]): UriItem[] {
  return uris.map((uri, index) => ({
    id: `init-${index}-${Date.now()}`,
    value: uri,
    isValid: true,
  }))
}

function transformFromUriItems(items: UriItem[]): string[] {
  return items.filter((item) => item.isValid).map((item) => item.value)
}

const SESSION_TTL_PRESETS = [
  { label: '30m', value: 1800 },
  { label: '1h', value: 3600 },
  { label: '2h', value: 7200 },
  { label: '4h', value: 14400 },
  { label: '8h', value: 28800 },
  { label: '12h', value: 43200 },
  { label: '24h', value: 86400 },
]

interface ClientAppFormPageProps {
  mode: 'create' | 'edit'
  realmId: string
  clientApp?: ClientAppItem
}

export function ClientAppFormPage({
  mode,
  realmId,
  clientApp,
}: ClientAppFormPageProps) {
  const isCreate = mode === 'create'
  const navigate = useNavigate()

  const handleCancel = () => {
    navigate({ to: '/$realmId/manage/client-apps', params: { realmId } })
  }

  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (data: CreateClientAppFormData | UpdateClientAppFormData) => {
      if (isCreate) {
        return createClientApp({
          path: { realmId },
          body: data as CreateClientAppFormData,
        }).then((response) => {
          if (response.error) throw response.error
          return response.data
        })
      }
      return updateClientApp({
        path: { realmId, clientAppId: clientApp!.id },
        body: data as UpdateClientAppFormData,
      }).then((response) => {
        if (response.error) throw response.error
        return response.data
      })
    },
    invalidateQueries: [queryKeys.clientAppsList(realmId)],
    onSuccess: (data) => {
      if (isCreate && data?.clientSecret) {
        toast.success('Client App created. Save your client secret — it will not be shown again.', {
          description: data.clientSecret,
          duration: 15000,
        })
      } else {
        toast.success(isCreate ? 'Client App created successfully' : 'Client App updated successfully')
      }
      navigate({ to: '/$realmId/manage/client-apps', params: { realmId } })
    },
  })

  const form = useAppForm({
    schema: isCreate ? createClientAppSchema : updateClientAppSchema,
    defaultValues: isCreate
      ? ({
          clientId: '',
          name: '',
          description: '',
          redirectUris: [],
          iconUrl: '',
          enabled: true,
          sessionTtlSeconds: 1800,
          sessionRenewalTtlSeconds: null,
          deviceCodeGrantEnabled: false,
        } as CreateClientAppFormData)
      : ({
          name: clientApp?.name ?? '',
          description: clientApp?.description ?? '',
          redirectUris: clientApp?.redirectUris ?? [],
          iconUrl: clientApp?.iconUrl ?? '',
          enabled: clientApp?.enabled ?? true,
          sessionTtlSeconds: clientApp?.sessionTtlSeconds ?? 1800,
          sessionRenewalTtlSeconds: clientApp?.sessionRenewalTtlSeconds ?? null,
          deviceCodeGrantEnabled: clientApp?.deviceCodeGrantEnabled ?? false,
          regenerateSecret: false,
        } as UpdateClientAppFormData),
    onSubmit: async ({ value }) => {
      await mutate(value)
    },
  })

  return (
    <div className="space-y-6" data-testid="client-app-form-page">
      <div className="flex items-center gap-4">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleCancel}
          data-testid="client-app-form-back-button"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div>
          <h1 className="text-2xl font-bold" data-testid="page-title">
            {isCreate ? 'Create Client App' : 'Edit Client App'}
          </h1>
          <p className="text-muted-foreground text-sm">
            {isCreate
              ? 'Fill in the details to create a new OAuth client application.'
              : 'Update the client application configuration.'}
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
          className="max-w-4xl space-y-6"
        >
          <Tabs defaultValue="basic" className="w-full">
            <TabsList className="w-full">
              <TabsTrigger value="basic" data-testid="tab-basic">Basic</TabsTrigger>
              <TabsTrigger value="redirect-uris" data-testid="tab-redirect-uris">Redirect URIs</TabsTrigger>
              <TabsTrigger value="security" data-testid="tab-security">Security</TabsTrigger>
              <TabsTrigger value="appearance" data-testid="tab-appearance">Appearance</TabsTrigger>
            </TabsList>

            <TabsContent value="basic" className="space-y-4 mt-4">
              {isCreate ? (
                <form.Field
                  name="clientId"
                  children={(field) => (
                    <div className="space-y-2">
                      <Label htmlFor="client-id">Client ID</Label>
                      <Input
                        id="client-id"
                        value={field.state.value ?? ''}
                        onChange={(e) => field.handleChange(e.target.value)}
                        placeholder="my-client-app"
                        data-testid="client-id-input"
                      />
                      {(field.state.meta.isTouched || form.state.isSubmitted) &&
                        field.state.meta.errors.length > 0 && (
                          <p className="text-sm text-destructive">
                            {getFieldErrorMessage(field.state.meta)}
                          </p>
                        )}
                    </div>
                  )}
                />
              ) : (
                <div className="space-y-2">
                  <Label>Client ID</Label>
                  <p className="text-sm font-mono bg-muted px-3 py-2 rounded-md" data-testid="client-id-display">
                    {clientApp?.clientId}
                  </p>
                </div>
              )}

              <form.Field
                name="name"
                children={(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="app-name">Name</Label>
                    <Input
                      id="app-name"
                      value={field.state.value ?? ''}
                      onChange={(e) => field.handleChange(e.target.value)}
                      data-testid="client-app-name-input"
                    />
                    {(field.state.meta.isTouched || form.state.isSubmitted) &&
                      field.state.meta.errors.length > 0 && (
                        <p className="text-sm text-destructive">
                          {getFieldErrorMessage(field.state.meta)}
                        </p>
                      )}
                  </div>
                )}
              />

              <form.Field
                name="description"
                children={(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="app-description">Description</Label>
                    <Textarea
                      id="app-description"
                      value={field.state.value ?? ''}
                      onChange={(e) => field.handleChange(e.target.value)}
                      rows={3}
                      placeholder="Optional description"
                      data-testid="client-app-description-input"
                    />
                  </div>
                )}
              />
            </TabsContent>

            <TabsContent value="redirect-uris" className="mt-4">
              <form.Field
                name="redirectUris"
                children={(field) => (
                  <RedirectUrisInput
                    value={transformToUriItems(field.state.value ?? [])}
                    onChange={(items) => field.handleChange(transformFromUriItems(items))}
                    label="Redirect URIs"
                    required
                    helpText="Allowed OAuth redirect URIs. Must be valid https:// or http:// URLs."
                    dataTestId="redirect-uris-input"
                  />
                )}
              />
            </TabsContent>

            <TabsContent value="security" className="space-y-4 mt-4">
              <form.Field
                name="enabled"
                children={(field) => (
                  <div className="flex items-center justify-between">
                    <Label htmlFor="app-enabled">Enabled</Label>
                    <Switch
                      id="app-enabled"
                      checked={field.state.value ?? true}
                      onCheckedChange={field.handleChange}
                      data-testid="client-app-enabled-switch"
                    />
                  </div>
                )}
              />

              <form.Field
                name="sessionTtlSeconds"
                children={(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="session-ttl">Session TTL (seconds)</Label>
                    <Input
                      id="session-ttl"
                      type="number"
                      value={field.state.value ?? 1800}
                      onChange={(e) => field.handleChange(Number(e.target.value))}
                      min={60}
                      max={86400}
                      data-testid="session-ttl-input"
                    />
                    <div className="flex flex-wrap gap-1">
                      {SESSION_TTL_PRESETS.map((preset) => (
                        <Button
                          key={preset.value}
                          type="button"
                          variant="outline"
                          size="sm"
                          className="h-7 text-xs"
                          onClick={() => field.handleChange(preset.value)}
                          data-testid={`session-ttl-preset-${preset.label}`}
                        >
                          {preset.label}
                        </Button>
                      ))}
                    </div>
                    {(field.state.meta.isTouched || form.state.isSubmitted) &&
                      field.state.meta.errors.length > 0 && (
                        <p className="text-sm text-destructive">
                          {getFieldErrorMessage(field.state.meta)}
                        </p>
                      )}
                  </div>
                )}
              />

              <form.Field
                name="sessionRenewalTtlSeconds"
                children={(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="session-renewal-ttl">Session Renewal TTL (seconds)</Label>
                    <Input
                      id="session-renewal-ttl"
                      type="number"
                      value={field.state.value ?? ''}
                      onChange={(e) => {
                        const val = e.target.value
                        field.handleChange(val === '' ? null : Number(val))
                      }}
                      placeholder="Optional — enables sliding sessions"
                      min={60}
                      max={604800}
                      data-testid="session-renewal-ttl-input"
                    />
                    {(field.state.meta.isTouched || form.state.isSubmitted) &&
                      field.state.meta.errors.length > 0 && (
                        <p className="text-sm text-destructive">
                          {getFieldErrorMessage(field.state.meta)}
                        </p>
                      )}
                  </div>
                )}
              />

              <form.Field
                name="deviceCodeGrantEnabled"
                children={(field) => (
                  <div className="flex items-center justify-between">
                    <Label htmlFor="device-code-grant">Device Code Grant</Label>
                    <Switch
                      id="device-code-grant"
                      checked={field.state.value ?? false}
                      onCheckedChange={field.handleChange}
                      data-testid="device-code-grant-switch"
                    />
                  </div>
                )}
              />

              {!isCreate && (
                <form.Field
                  name="regenerateSecret"
                  children={(field) => (
                    <div className="flex items-center justify-between pt-4 border-t">
                      <div>
                        <Label htmlFor="regenerate-secret">Regenerate Secret</Label>
                        <p className="text-xs text-muted-foreground">
                          The old secret will be invalidated immediately.
                        </p>
                      </div>
                      <Switch
                        id="regenerate-secret"
                        checked={field.state.value ?? false}
                        onCheckedChange={field.handleChange}
                        data-testid="regenerate-secret-switch"
                      />
                    </div>
                  )}
                />
              )}
            </TabsContent>

            <TabsContent value="appearance" className="space-y-4 mt-4">
              <form.Field
                name="iconUrl"
                children={(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="icon-url">Icon URL</Label>
                    <Input
                      id="icon-url"
                      value={field.state.value ?? ''}
                      onChange={(e) => field.handleChange(e.target.value)}
                      placeholder="https://example.com/icon.png"
                      data-testid="icon-url-input"
                    />
                  </div>
                )}
              />
            </TabsContent>
          </Tabs>

          <div className="flex justify-end gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={handleCancel}
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
