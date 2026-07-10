import { useEffect, useRef, useState } from 'react'
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
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { TextField } from '@/components/shared/form-fields/text-field'
import { formatDateTimeShort } from '@/lib/date-utils'
import { m } from '@/paraglide/messages'

export interface CustomDomainConfigFormProps {
  /** Realm the config belongs to (passed through for logging/query keys). */
  realmId: string
  /** Initial form values. Use `emptyCustomDomainConfig()` when nothing is configured yet. */
  initialConfig: CustomDomainConfigFormValues
  /** Whether an unpublished draft exists on the backend (shows the draft notice). */
  hasDraft?: boolean
  /** Whether a previous version can be restored. Gates the restore button. */
  hasPrevious?: boolean
  /** Disables all inputs + action buttons (e.g. missing `settings.manage`). */
  disabled?: boolean

  /** Herald-owned hostname tenants CNAME their custom login domain to (GET response field). */
  cnameTarget: string
  /** Live CNAME/TLS status of the published hostname (GET response field, may be null). */
  status: CustomDomainStatus | null

  /**
   * Persist the current form values as a draft. Receives the already-normalized
   * PUT `/draft` request body so FE-D03 can pass it straight to the generated
   * client. Returns a promise so the form can track in-flight state; rejections
   * surface as the save error (e.g. 409 conflict) via the parent mutation.
   */
  onSaveDraft: (config: CustomDomainConfigFormValues) => void | Promise<void>
  /** Publish the current form values (writes `settings`). */
  onPublish: (config: CustomDomainConfigFormValues) => void | Promise<void>
  /** Discard the saved draft and reset the editor to the published config. */
  onDiscardDraft: () => void | Promise<void>
  /** Restore the previous published config. Requires `hasPrevious`. */
  onRestore: () => void | Promise<void>

  /** Manually re-fetch the CNAME/TLS status of the published hostname. */
  onRefreshStatus?: () => void

  /** In-flight flags driven by the parent's mutations. */
  isSavingDraft?: boolean
  isPublishing?: boolean
  isDiscarding?: boolean
  isRestoring?: boolean
  isRefreshing?: boolean
}

export function CustomDomainConfigForm({
  realmId: _realmId,
  initialConfig,
  hasDraft = false,
  hasPrevious = false,
  disabled = false,
  cnameTarget,
  status,
  onSaveDraft,
  onPublish,
  onDiscardDraft,
  onRestore,
  onRefreshStatus,
  isSavingDraft = false,
  isPublishing = false,
  isDiscarding = false,
  isRestoring = false,
  isRefreshing = false,
}: CustomDomainConfigFormProps) {
  const [restoreDialogOpen, setRestoreDialogOpen] = useState(false)

  const form = useAppForm({
    schema: customDomainConfigSchema,
    defaultValues: initialConfig,
    onSubmit: async ({ value }) => {
      // The primary submit target is "save draft". Publish/discard/restore are
      // explicit buttons with their own handlers; keeping a single default
      // submit matches the existing config-form pattern (Enter submits draft).
      await onSaveDraft(value)
    },
  })

  // Keep the form in sync with the persisted source. `initialConfig` is derived
  // upstream from `draft ?? published`, so it only changes value when the
  // backend state changes externally (e.g. after discard the draft is gone and
  // the source flips back to `published`, or after restore/publish). We compare
  // by value (not reference) because the parent rebuilds the object on every
  // render, and only reseed the form when the source value actually differs
  // from the last one we applied — so in-flight edits are never wiped.
  const sourceKey = JSON.stringify(initialConfig)
  const lastSyncedSourceRef = useRef(sourceKey)
  useEffect(() => {
    if (sourceKey !== lastSyncedSourceRef.current) {
      lastSyncedSourceRef.current = sourceKey
      form.reset(initialConfig)
    }
  }, [sourceKey, initialConfig, form])

  // Subscribe to the live form values + dirty flag so the draft notice updates
  // as the user types without re-rendering on unrelated state changes.
  const values = useStore(form.store, (state) => state.values)
  const isDirty = useStore(form.store, (state) => state.isDirty)

  const showDraftNotice = hasDraft || isDirty
  const effectiveHostname = values.hostname?.trim() || null

  // --- Action handlers ---------------------------------------------------------
  // Each action reads the *current* form values (not stale closure values) via
  // form.store so the parent always receives the latest edit. Validation is
  // delegated to the schema; on invalid values the action is skipped.
  const handlePublish = () => {
    if (disabled) return
    void onPublish(values)
  }

  const handleDiscardDraft = () => {
    if (disabled) return
    void onDiscardDraft()
  }

  const handleRestoreConfirm = () => {
    setRestoreDialogOpen(false)
    void onRestore()
  }

  const handleRefreshStatus = () => {
    if (disabled || isRefreshing) return
    onRefreshStatus?.()
  }

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
                    onClick={handleRefreshStatus}
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

              {showDraftNotice && (
                <p className="text-sm text-amber-600" data-testid="custom-domain-draft-notice">
                  {m['settings.custom_domain.draft_notice']()}
                </p>
              )}

              {/* Action buttons */}
              <div className="flex flex-wrap gap-2 pt-2">
                <Button
                  type="submit"
                  disabled={disabled || isSavingDraft}
                  data-testid="custom-domain-save-draft"
                >
                  {isSavingDraft
                    ? m['settings.custom_domain.saving']()
                    : m['settings.custom_domain.save_draft']()}
                </Button>
                <Button
                  type="button"
                  disabled={disabled || isPublishing}
                  data-testid="custom-domain-publish"
                  onClick={handlePublish}
                >
                  {isPublishing
                    ? m['settings.custom_domain.publishing']()
                    : m['settings.custom_domain.publish']()}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  disabled={disabled || isDiscarding || !hasDraft}
                  data-testid="custom-domain-discard-draft"
                  onClick={handleDiscardDraft}
                >
                  {isDiscarding
                    ? m['settings.custom_domain.discarding']()
                    : m['settings.custom_domain.discard_draft']()}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  disabled={disabled || isRestoring || !hasPrevious}
                  data-testid="custom-domain-restore"
                  onClick={() => setRestoreDialogOpen(true)}
                >
                  {isRestoring
                    ? m['settings.custom_domain.restoring']()
                    : m['settings.custom_domain.restore']()}
                </Button>
              </div>
            </form>
          </AppForm>
        </CardContent>
      </Card>

      {/* ============ Restore confirmation dialog ============ */}
      <AlertDialog open={restoreDialogOpen} onOpenChange={setRestoreDialogOpen}>
        <AlertDialogContent data-testid="custom-domain-restore-dialog">
          <AlertDialogHeader>
            <AlertDialogTitle>
              {m['settings.custom_domain.restore_dialog_title']()}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {m['settings.custom_domain.restore_dialog_description']()}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={isRestoring}>{m['common.cancel']()}</AlertDialogCancel>
            <AlertDialogAction
              disabled={isRestoring}
              onClick={(e) => {
                e.preventDefault()
                handleRestoreConfirm()
              }}
              data-testid="custom-domain-restore-confirm"
            >
              {isRestoring
                ? m['settings.custom_domain.restoring']()
                : m['settings.custom_domain.restore']()}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  )
}
