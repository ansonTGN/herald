import { useEffect, useRef } from 'react'
import { useStore } from '@tanstack/react-form'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import {
  customDomainConfigSchema,
  type CustomDomainConfigForm as CustomDomainConfigFormValues,
} from '@/lib/schemas/realm-config'
import type { CustomDomainStatus } from '@/lib/api-generated'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { TextField } from '@/components/shared/form-fields/text-field'
import { formatDateTimeShort } from '@/lib/date-utils'
import { m } from '@/paraglide/messages'

export interface CustomDomainConfigFormProps {
  /** Realm the config belongs to (passed through for logging/query keys). */
  realmId: string
  /** Initial form values. Use `emptyCustomDomainConfig()` when nothing is configured yet. */
  initialConfig: CustomDomainConfigFormValues
  /** Disables all inputs + action buttons (e.g. missing `settings.manage`). */
  disabled?: boolean

  /** Herald-owned hostname tenants CNAME their custom login domain to (GET response field). */
  cnameTarget: string
  /** Live CNAME/TLS status of the configured hostname (GET response field, may be null). */
  status: CustomDomainStatus | null

  /**
   * Persist the current form values (writes the host→realm mapping + settings
   * in one step). Receives the already-normalized PUT request body. Returns a
   * promise so the form can track in-flight state; rejections surface as the
   * save error (e.g. 409 conflict) via the parent mutation.
   */
  onSave: (config: CustomDomainConfigFormValues) => void | Promise<void>

  /** Manually re-fetch the CNAME/TLS status of the configured hostname. */
  onRefreshStatus?: () => void

  /** In-flight flags driven by the parent's mutations. */
  isSaving?: boolean
  isRefreshing?: boolean
}

export function CustomDomainConfigForm({
  realmId: _realmId,
  initialConfig,
  disabled = false,
  cnameTarget,
  status,
  onSave,
  onRefreshStatus,
  isSaving = false,
  isRefreshing = false,
}: CustomDomainConfigFormProps) {
  const form = useAppForm({
    schema: customDomainConfigSchema,
    defaultValues: initialConfig,
    onSubmit: async ({ value }) => {
      await onSave(value)
    },
  })

  // Keep the form in sync with the persisted source. `initialConfig` is the
  // published config from the backend, so it only changes value when the
  // backend state changes externally (e.g. after save). We compare by value
  // (not reference) because the parent rebuilds the object on every render,
  // and only reseed the form when the source value actually differs from the
  // last one we applied — so in-flight edits are never wiped.
  const sourceKey = JSON.stringify(initialConfig)
  const lastSyncedSourceRef = useRef(sourceKey)
  useEffect(() => {
    if (sourceKey !== lastSyncedSourceRef.current) {
      lastSyncedSourceRef.current = sourceKey
      form.reset(initialConfig)
    }
  }, [sourceKey, initialConfig, form])

  // Subscribe to the live form values so the CNAME guidance updates as the
  // user types without re-rendering on unrelated state changes.
  const values = useStore(form.store, (state) => state.values)

  const effectiveHostname = values.hostname?.trim() || null

  const cnameVerified = status?.cnameVerified ?? false
  const tlsReady = status?.tlsReady ?? false
  const checkedAt = status?.checkedAt ?? null

  return (
    <div className="grid gap-6">
      <Card>
        <CardHeader>
          <CardTitle>{m['settings.custom_domain.title']()}</CardTitle>
          <CardDescription>{m['settings.custom_domain.description']()}</CardDescription>
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
              <TextField
                form={form}
                name="hostname"
                label={m['settings.custom_domain.hostname_label']()}
                inputId="custom-domain-hostname"
                dataTestId="custom-domain-hostname"
                placeholder="login.acme.com"
                disabled={disabled}
                helpText={m['settings.custom_domain.hostname_help']()}
              />

              {/* CNAME guidance: shows the Herald-owned target hostname tenants
                  must point their login domain at. The hostname example uses the
                  current edit value when present, falling back to a placeholder. */}
              <div
                className="space-y-1 rounded-md border bg-muted/40 p-4 text-sm"
                data-testid="custom-domain-cname-guidance"
              >
                <p className="font-medium">
                  {m['settings.custom_domain.cname_guidance']({
                    hostname: effectiveHostname ?? 'your-domain',
                    cnameTarget,
                  })}
                </p>
              </div>

              {/* Effective status: CNAME/TLS readiness badges + last-checked time + manual refresh.
                  These come from the GET response `status` field (threaded from the parent); the
                  form does not fetch status itself. */}
              <div className="space-y-2 rounded-md border p-4">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge
                    variant={cnameVerified ? 'default' : 'secondary'}
                    data-testid="custom-domain-status-cname"
                  >
                    {cnameVerified
                      ? m['settings.custom_domain.status_cname_verified']()
                      : m['settings.custom_domain.status_cname_pending']()}
                  </Badge>
                  <Badge
                    variant={tlsReady ? 'default' : 'secondary'}
                    data-testid="custom-domain-status-tls"
                  >
                    {tlsReady
                      ? m['settings.custom_domain.status_tls_ready']()
                      : m['settings.custom_domain.status_tls_pending']()}
                  </Badge>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    disabled={disabled || isRefreshing}
                    onClick={onRefreshStatus}
                    data-testid="custom-domain-refresh-status"
                  >
                    {isRefreshing
                      ? m['settings.custom_domain.refreshing_status']()
                      : m['settings.custom_domain.refresh_status']()}
                  </Button>
                </div>
                {checkedAt && (
                  <p className="text-xs text-muted-foreground">
                    {m['settings.custom_domain.status_checked_at']({
                      time: formatDateTimeShort(checkedAt),
                    })}
                  </p>
                )}
              </div>

              {/* Action buttons */}
              <div className="flex flex-wrap gap-2 pt-2">
                <Button
                  type="submit"
                  disabled={disabled || isSaving}
                  data-testid="custom-domain-save"
                >
                  {isSaving
                    ? m['settings.custom_domain.saving']()
                    : m['settings.custom_domain.save']()}
                </Button>
              </div>
            </form>
          </AppForm>
        </CardContent>
      </Card>
    </div>
  )
}
