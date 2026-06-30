import { createFileRoute } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Info, Loader2 } from 'lucide-react'
import {
  queryKeys,
  pointsDefaultConfigQueryOptions,
  updatePointsDefaultConfigMutation,
} from '@/data/query-options'
import {
  pointsDefaultConfigSchema,
  type PointsDefaultConfigFormData,
} from '@/lib/schemas/points-forms'
import { MultiWindowQuotaEditor } from '@/components/billing/MultiWindowQuotaEditor'
import { useAuth } from '@/hooks/use-auth'
import { PERMISSION } from '@/lib/constants/auth-constants'
import { AccessDenied } from '@/components/shared'
import { m } from '@/paraglide/messages'

export const Route = createFileRoute('/$realmId/manage/points/default-config')({
  component: RealmConfigPage,
})

// Exported directly (not via Route.component) so tests can mount the page without
// the TanStack Router autoCodeSplitting Lazy wrapper, which never resolves
// outside a real Router context. Sibling pages (e.g. EntitlementMappingsPage)
// are already directly importable; this mirrors that pattern.
export function RealmConfigPage() {
  const { realmId } = Route.useParams()
  const queryClient = useQueryClient()
  const auth = useAuth()

  // Permission checks (defense-in-depth mirroring settings.tsx; backend
  // realm_configs.rs + design §3.3/§4.5 gate these endpoints on
  // settings.view/settings.manage, NOT points.*).
  const canViewConfig = auth.permissions?.includes(PERMISSION.SETTINGS_VIEW) ?? false
  const canManageConfig = auth.permissions?.includes(PERMISSION.SETTINGS_MANAGE) ?? false

  const periodTypeLabels = {
    once: m['points.default_config_period_once'](),
    daily: m['points.default_config_period_daily'](),
    weekly: m['points.default_config_period_weekly'](),
    monthly: m['points.default_config_period_monthly'](),
  }

  const {
    data: config,
    isLoading,
    error,
  } = useQuery({ ...pointsDefaultConfigQueryOptions(realmId), enabled: canViewConfig })

  // Treat 404 (no config yet) as non-error: use defaults
  const isNotFound = error && /404|not found/i.test((error as Error).message || '')
  // The query layer maps a 403 to `new Error('Insufficient permissions')`; the
  // raw status is not carried on the Error instance, so match the known text.
  const isForbidden = error && /Insufficient permissions/i.test((error as Error).message || '')
  const effectiveConfig = config ?? null

  const updateMutation = useMutation({
    mutationFn: (data: PointsDefaultConfigFormData) =>
      updatePointsDefaultConfigMutation(realmId, {
        registrationBonusPoints: data.registrationBonusPoints,
        freePeriodicPointsAmount: data.freePeriodicPointsAmount,
        freePeriodicGrantPeriodType: data.freePeriodicGrantPeriodType,
        freePeriodicValidityDays: data.freePeriodicValidityDays,
        freePeriodicQuotaWindows: data.freePeriodicQuotaWindows,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.pointsDefaultConfig(realmId) })
      toast.success(m['points.default_config_saved']())
    },
    onError: (error) => {
      console.error('Failed to update realm config:', error)
      toast.error(error.message || m['points.default_config_save_failed']())
    },
  })

  const form = useAppForm({
    schema: pointsDefaultConfigSchema,
    defaultValues: effectiveConfig
      ? {
          ...effectiveConfig,
          freePeriodicGrantPeriodType: effectiveConfig.freePeriodicGrantPeriodType as
            | 'once'
            | 'daily'
            | 'weekly'
            | 'monthly',
          freePeriodicQuotaWindows: effectiveConfig.freePeriodicQuotaWindows ?? [],
        }
      : {
          registrationBonusPoints: 1000,
          freePeriodicPointsAmount: 50,
          freePeriodicGrantPeriodType: 'daily',
          freePeriodicValidityDays: 1,
          freePeriodicQuotaWindows: [],
        },
    onSubmit: async ({ value }) => {
      // Defense-in-depth: the Save button is also disabled when the user lacks
      // SETTINGS_MANAGE, but guard here too in case submit is triggered anyway.
      if (!canManageConfig) return
      // The mutation's `onError` surfaces the failure toast and logs the error.
      // Swallow the rejected promise so it doesn't propagate as an unhandled
      // rejection (per vitest config, rejections are expected to be handled in
      // components). Sibling forms share this latent leak; see FE-T06 handoff.
      await updateMutation.mutateAsync(value).catch(() => {})
    },
  })

  if (!canViewConfig) {
    return <AccessDenied message={m['error.access_denied']()} />
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error && isForbidden) {
    return <AccessDenied message={m['error.access_denied']()} />
  }

  if (error && !isNotFound) {
    return (
      <Alert variant="destructive">
        <AlertDescription>
          {m['points.default_config_load_failed']({ message: (error as Error).message })}
        </AlertDescription>
      </Alert>
    )
  }

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-semibold">{m['points.default_config_page_title']()}</h1>

      <Alert>
        <Info className="h-4 w-4" />
        <AlertDescription>{m['points.default_config_info']()}</AlertDescription>
      </Alert>

      <Card data-testid="points-default-config-form" aria-labelledby="points-default-config-title">
        <CardHeader>
          <CardTitle id="points-default-config-title">
            {m['points.default_config_card_title']()}
          </CardTitle>
          <CardDescription>{m['points.default_config_card_description']()}</CardDescription>
        </CardHeader>
        <CardContent>
          <AppForm>
            <form
              onSubmit={(e) => {
                e.preventDefault()
                form.handleSubmit()
              }}
              className="space-y-6"
              aria-label="Points default config form"
            >
              <form.Field name="registrationBonusPoints">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name} id={`${field.name}-label`}>
                      {m['points.default_config_registration_bonus_label']()}
                    </Label>
                    <Input
                      id={field.name}
                      type="number"
                      min="0"
                      step="1"
                      value={field.state.value}
                      onChange={(e) => field.handleChange(parseInt(e.target.value) || 0)}
                      placeholder="1000"
                      data-testid="registration-bonus-points-input"
                      aria-labelledby={`${field.name}-label`}
                      aria-describedby={`${field.name}-description ${field.state.meta.errors.length > 0 ? `${field.name}-error` : ''}`}
                      aria-invalid={field.state.meta.errors.length > 0}
                      aria-required="true"
                      disabled={!canManageConfig || updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      {m['points.default_config_registration_bonus_help']()}
                    </p>
                    {field.state.meta.errors.length > 0 && (
                      <p
                        id={`${field.name}-error`}
                        className="text-sm text-destructive"
                        data-testid="registration-bonus-points-error"
                        role="alert"
                      >
                        {(field.state.meta.errors[0] as { message?: string })?.message ||
                          String(field.state.meta.errors[0])}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              <form.Field name="freePeriodicGrantPeriodType">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name} id={`${field.name}-label`}>
                      {m['points.default_config_grant_period_type_label']()}
                    </Label>
                    <Select
                      value={field.state.value}
                      onValueChange={(value) =>
                        field.handleChange(value as 'once' | 'daily' | 'weekly' | 'monthly')
                      }
                      disabled={!canManageConfig || updateMutation.isPending}
                    >
                      <SelectTrigger
                        id={field.name}
                        data-testid="grant-period-type-select"
                        aria-labelledby={`${field.name}-label`}
                        aria-describedby={`${field.name}-description ${field.state.meta.errors.length > 0 ? `${field.name}-error` : ''}`}
                        aria-invalid={field.state.meta.errors.length > 0}
                        aria-required="true"
                      >
                        <SelectValue
                          placeholder={m['points.default_config_grant_period_type_placeholder']()}
                        />
                      </SelectTrigger>
                      <SelectContent>
                        {Object.entries(periodTypeLabels).map(([value, label]) => (
                          <SelectItem key={value} value={value}>
                            {label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      {m['points.default_config_grant_period_help']()}
                    </p>
                    {field.state.meta.errors.length > 0 && (
                      <p
                        id={`${field.name}-error`}
                        className="text-sm text-destructive"
                        data-testid="grant-period-type-error"
                        role="alert"
                      >
                        {(field.state.meta.errors[0] as { message?: string })?.message ||
                          String(field.state.meta.errors[0])}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              <form.Field name="freePeriodicPointsAmount">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name} id={`${field.name}-label`}>
                      {m['points.default_config_periodic_amount_label']()}
                    </Label>
                    <Input
                      id={field.name}
                      type="number"
                      min="0"
                      step="1"
                      value={field.state.value}
                      onChange={(e) => field.handleChange(parseInt(e.target.value) || 0)}
                      placeholder="50"
                      data-testid="free-periodic-points-amount-input"
                      aria-labelledby={`${field.name}-label`}
                      aria-describedby={`${field.name}-description ${field.state.meta.errors.length > 0 ? `${field.name}-error` : ''}`}
                      aria-invalid={field.state.meta.errors.length > 0}
                      aria-required="true"
                      disabled={!canManageConfig || updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      {m['points.default_config_periodic_amount_help']()}
                    </p>
                    {field.state.meta.errors.length > 0 && (
                      <p
                        id={`${field.name}-error`}
                        className="text-sm text-destructive"
                        data-testid="free-periodic-points-amount-error"
                        role="alert"
                      >
                        {(field.state.meta.errors[0] as { message?: string })?.message ||
                          String(field.state.meta.errors[0])}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              <form.Field name="freePeriodicValidityDays">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name} id={`${field.name}-label`}>
                      {m['points.default_config_validity_days_label']()}
                    </Label>
                    <Input
                      id={field.name}
                      type="number"
                      min="0"
                      step="1"
                      value={field.state.value}
                      onChange={(e) => field.handleChange(parseInt(e.target.value) || 0)}
                      placeholder="1"
                      data-testid="free-periodic-validity-days-input"
                      aria-labelledby={`${field.name}-label`}
                      aria-describedby={`${field.name}-description ${field.state.meta.errors.length > 0 ? `${field.name}-error` : ''}`}
                      aria-invalid={field.state.meta.errors.length > 0}
                      aria-required="true"
                      disabled={!canManageConfig || updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      {m['points.default_config_validity_days_help']()}
                    </p>
                    {field.state.meta.errors.length > 0 && (
                      <p
                        id={`${field.name}-error`}
                        className="text-sm text-destructive"
                        data-testid="free-periodic-validity-days-error"
                        role="alert"
                      >
                        {(field.state.meta.errors[0] as { message?: string })?.message ||
                          String(field.state.meta.errors[0])}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              <form.Field name="freePeriodicQuotaWindows">
                {(field) => (
                  <div className="space-y-2">
                    <Label
                      id={`${field.name}-label`}
                      className="text-xs font-medium text-muted-foreground"
                    >
                      {m['points.quota_editor_title']()}
                    </Label>
                    <MultiWindowQuotaEditor
                      value={field.state.value ?? []}
                      onChange={(next) => field.handleChange(next)}
                      disabled={!canManageConfig || updateMutation.isPending}
                      context="realm-default"
                      testIdPrefix="realm-default-window"
                    />
                  </div>
                )}
              </form.Field>

              {/* Action Buttons */}
              <div className="flex justify-end pt-4">
                <form.Subscribe
                  selector={(state) => ({
                    canSubmit: state.canSubmit,
                    isSubmitting: state.isSubmitting,
                  })}
                >
                  {(state) => (
                    <Button
                      type="submit"
                      disabled={
                        !canManageConfig ||
                        !state.canSubmit ||
                        state.isSubmitting ||
                        updateMutation.isPending
                      }
                      data-testid="save-config-button"
                      aria-label={
                        state.isSubmitting || updateMutation.isPending
                          ? m['points.default_config_saving_button']()
                          : m['points.default_config_save_button']()
                      }
                      aria-busy={state.isSubmitting || updateMutation.isPending}
                    >
                      {state.isSubmitting || updateMutation.isPending
                        ? m['points.default_config_saving_button']()
                        : m['points.default_config_save_button']()}
                    </Button>
                  )}
                </form.Subscribe>
              </div>
            </form>
          </AppForm>
        </CardContent>
      </Card>
    </div>
  )
}
