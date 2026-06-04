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
  realmConfigQueryOptions,
  updateRealmDefaultConfigMutation,
} from '@/data/query-options'
import { realmConfigSchema, type RealmConfigFormData } from '@/lib/schemas/points-forms'
import { m } from '@/paraglide/messages'

export const Route = createFileRoute('/$realmId/manage/points/realm-config')({
  component: RealmConfigPage,
})

function RealmConfigPage() {
  const { realmId } = Route.useParams()
  const queryClient = useQueryClient()

  const periodTypeLabels = {
    once: m['points.realm_config_period_once'](),
    daily: m['points.realm_config_period_daily'](),
    weekly: m['points.realm_config_period_weekly'](),
    monthly: m['points.realm_config_period_monthly'](),
  }

  const { data: config, isLoading, error } = useQuery(realmConfigQueryOptions(realmId))

  // Treat 404 (no config yet) as non-error: use defaults
  const isNotFound = error && /404|not found/i.test((error as Error).message || '')
  const effectiveConfig = config ?? null

  const updateMutation = useMutation({
    mutationFn: (data: RealmConfigFormData) =>
      updateRealmDefaultConfigMutation(realmId, {
        registrationBonusPoints: data.registrationBonusPoints,
        freePeriodicPointsAmount: data.freePeriodicPointsAmount,
        freePeriodicGrantPeriodType: data.freePeriodicGrantPeriodType,
        freePeriodicValidityDays: data.freePeriodicValidityDays,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.realmConfig(realmId) })
      toast.success(m['points.realm_config_saved']())
    },
    onError: (error) => {
      console.error('Failed to update realm config:', error)
      toast.error(error.message || m['points.realm_config_save_failed']())
    },
  })

  const form = useAppForm({
    schema: realmConfigSchema,
    defaultValues: effectiveConfig
      ? {
          ...effectiveConfig,
          freePeriodicGrantPeriodType: effectiveConfig.freePeriodicGrantPeriodType as
            | 'once'
            | 'daily'
            | 'weekly'
            | 'monthly',
        }
      : {
          registrationBonusPoints: 1000,
          freePeriodicPointsAmount: 50,
          freePeriodicGrantPeriodType: 'daily',
          freePeriodicValidityDays: 1,
        },
    onSubmit: async ({ value }) => {
      await updateMutation.mutateAsync(value)
    },
  })

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error && !isNotFound) {
    return (
      <Alert variant="destructive">
        <AlertDescription>
          {m['points.realm_config_load_failed']({ message: (error as Error).message })}
        </AlertDescription>
      </Alert>
    )
  }

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-semibold">{m['points.realm_config_page_title']()}</h1>

      <Alert>
        <Info className="h-4 w-4" />
        <AlertDescription>{m['points.realm_config_info']()}</AlertDescription>
      </Alert>

      <Card data-testid="realm-config-form" aria-labelledby="realm-config-title">
        <CardHeader>
          <CardTitle id="realm-config-title">{m['points.realm_config_card_title']()}</CardTitle>
          <CardDescription>{m['points.realm_config_card_description']()}</CardDescription>
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
                      {m['points.realm_config_registration_bonus_label']()}
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
                      disabled={updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      {m['points.realm_config_registration_bonus_help']()}
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
                      {m['points.realm_config_grant_period_type_label']()}
                    </Label>
                    <Select
                      value={field.state.value}
                      onValueChange={(value) =>
                        field.handleChange(value as 'once' | 'daily' | 'weekly' | 'monthly')
                      }
                      disabled={updateMutation.isPending}
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
                          placeholder={m['points.realm_config_grant_period_type_placeholder']()}
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
                      {m['points.realm_config_grant_period_help']()}
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
                      {m['points.realm_config_periodic_amount_label']()}
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
                      disabled={updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      {m['points.realm_config_periodic_amount_help']()}
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
                      {m['points.realm_config_validity_days_label']()}
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
                      disabled={updateMutation.isPending}
                    />
                    <p id={`${field.name}-description`} className="text-xs text-muted-foreground">
                      {m['points.realm_config_validity_days_help']()}
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
                      disabled={!state.canSubmit || state.isSubmitting || updateMutation.isPending}
                      data-testid="save-config-button"
                      aria-label={
                        state.isSubmitting || updateMutation.isPending
                          ? m['points.realm_config_saving_button']()
                          : m['points.realm_config_save_button']()
                      }
                      aria-busy={state.isSubmitting || updateMutation.isPending}
                    >
                      {state.isSubmitting || updateMutation.isPending
                        ? m['points.realm_config_saving_button']()
                        : m['points.realm_config_save_button']()}
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
