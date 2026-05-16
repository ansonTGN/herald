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

export const Route = createFileRoute('/$realmId/manage/points/realm-config')({
  component: RealmConfigPage,
})

function RealmConfigPage() {
  const { realmId } = Route.useParams()
  const queryClient = useQueryClient()

  const periodTypeLabels = {
    once: 'One-time grant',
    daily: 'Daily grant',
    weekly: 'Weekly grant',
    monthly: 'Monthly grant',
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
      toast.success('Configuration updated')
    },
    onError: (error) => {
      console.error('Failed to update realm config:', error)
      toast.error(error.message || 'Failed to update config, please retry')
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
        <AlertDescription>Failed to load config: {(error as Error).message}</AlertDescription>
      </Alert>
    )
  }

  return (
    <div className="space-y-6">
      <h1 className="text-xl font-semibold">Points Default Configuration</h1>

      <Alert>
        <Info className="h-4 w-4" />
        <AlertDescription>
          This config only affects newly registered users. Existing users are unaffected.
        </AlertDescription>
      </Alert>

      <Card data-testid="realm-config-form" aria-labelledby="realm-config-title">
        <CardHeader>
          <CardTitle id="realm-config-title">Default Configuration</CardTitle>
          <CardDescription>Set points reward rules for free users</CardDescription>
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
                      Registration Bonus Points *
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
                      One-time bonus points granted on registration
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
                      Grant Period Type *
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
                        <SelectValue placeholder="Select period type" />
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
                      Grant period for free user points
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
                      Periodic Points Amount *
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
                      Points granted automatically per period
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
                      Periodic Points Validity (Days) *
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
                      Validity period for periodic points; expired points are removed (set 0 for
                      one-time period to make permanent)
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
                          ? 'Saving configuration...'
                          : 'Save Configuration'
                      }
                      aria-busy={state.isSubmitting || updateMutation.isPending}
                    >
                      {state.isSubmitting || updateMutation.isPending
                        ? 'Saving...'
                        : 'Save Configuration'}
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
