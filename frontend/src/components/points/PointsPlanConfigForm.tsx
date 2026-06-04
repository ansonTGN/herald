import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
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
import { Switch } from '@/components/ui/switch'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import type { PointsPlanConfigResponse } from '@/lib/api-generated'
import type { PointsPlanConfigFormData } from '@/lib/schemas/points-forms'
import { pointsPlanConfigSchema } from '@/lib/schemas/points-forms'
import { m } from '@/paraglide/messages'

interface PointsPlanConfigFormProps {
  config?: PointsPlanConfigResponse | null
  plans: Array<{ id: string; name: string; title: string }>
  onSubmit: (data: PointsPlanConfigFormData) => void
  onCancel: () => void
  isSubmitting?: boolean
  showTitle?: boolean
}

export function PointsPlanConfigForm({
  config,
  plans,
  onSubmit,
  onCancel,
  isSubmitting = false,
  showTitle = true,
}: PointsPlanConfigFormProps) {
  const isEdit = !!config

  const form = useAppForm({
    schema: pointsPlanConfigSchema,
    defaultValues: {
      planId: config?.planId || '',
      pointsPerPeriod: config?.pointsPerPeriod || 0,
      grantOnSubscribe: config?.grantOnSubscribe ?? true,
      grantPeriodType:
        (config?.grantPeriodType as 'once' | 'daily' | 'weekly' | 'monthly') || 'monthly',
      maxPeriods: config?.maxPeriods || null,
      validityDays: config?.validityDays || 30,
    },
    onSubmit: async ({ value }) => {
      onSubmit(value)
    },
  })

  return (
    <Card data-testid="points-plan-config-form">
      {showTitle && (
        <CardHeader>
          <CardTitle>
            {isEdit ? m['points.plan_config_edit_title']() : m['points.plan_config_create_title']()}
          </CardTitle>
        </CardHeader>
      )}
      <CardContent>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            form.handleSubmit()
          }}
        >
          <AppForm>
            <div className="space-y-6">
              {/* Plan Selection */}
              <form.Field name="planId">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name}>{m['points.plan_config_form_plan_label']()}</Label>
                    <Select
                      value={field.state.value}
                      onValueChange={field.handleChange}
                      disabled={isEdit}
                      data-testid="plan-select"
                    >
                      <SelectTrigger id={field.name}>
                        <SelectValue
                          placeholder={m['points.plan_config_form_plan_placeholder']()}
                        />
                      </SelectTrigger>
                      <SelectContent>
                        {plans.map((plan) => (
                          <SelectItem key={plan.id} value={plan.id}>
                            {plan.title || plan.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {isEdit && (
                      <p className="text-xs text-muted-foreground">
                        {m['points.plan_config_form_plan_immutable']()}
                      </p>
                    )}
                  </div>
                )}
              </form.Field>

              {/* Points per Period */}
              <form.Field name="pointsPerPeriod">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name}>
                      {m['points.plan_config_form_points_per_period_label']()}
                    </Label>
                    <Input
                      id={field.name}
                      type="number"
                      min="0"
                      value={field.state.value}
                      onChange={(e) => field.handleChange(parseInt(e.target.value) || 0)}
                      placeholder="1000"
                      data-testid="points-per-period"
                    />
                    <p className="text-xs text-muted-foreground">
                      {m['points.plan_config_form_points_per_period_help']()}
                    </p>
                  </div>
                )}
              </form.Field>

              {/* Grant Settings */}
              <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <Label htmlFor="grant-on-subscribe">
                      {m['points.plan_config_form_grant_on_subscribe_label']()}
                    </Label>
                    <p className="text-xs text-muted-foreground">
                      {m['points.plan_config_form_grant_on_subscribe_help']()}
                    </p>
                  </div>
                  <form.Field name="grantOnSubscribe">
                    {(field) => (
                      <Switch
                        id="grant-on-subscribe"
                        checked={field.state.value}
                        onCheckedChange={field.handleChange}
                        data-testid="grant-on-subscribe"
                      />
                    )}
                  </form.Field>
                </div>

                <form.Field name="grantPeriodType">
                  {(field) => (
                    <div className="space-y-2">
                      <Label htmlFor={field.name}>
                        {m['points.plan_config_form_grant_period_label']()}
                      </Label>
                      <Select
                        value={field.state.value}
                        onValueChange={(value) =>
                          field.handleChange(value as 'once' | 'daily' | 'weekly' | 'monthly')
                        }
                      >
                        <SelectTrigger id={field.name} data-testid="grant-period-type">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="once">
                            {m['points.plan_config_form_period_once']()}
                          </SelectItem>
                          <SelectItem value="daily">
                            {m['points.plan_config_form_period_daily']()}
                          </SelectItem>
                          <SelectItem value="weekly">
                            {m['points.plan_config_form_period_weekly']()}
                          </SelectItem>
                          <SelectItem value="monthly">
                            {m['points.plan_config_form_period_monthly']()}
                          </SelectItem>
                        </SelectContent>
                      </Select>
                      <p className="text-xs text-muted-foreground">
                        {m['points.plan_config_form_grant_period_help']()}
                      </p>
                    </div>
                  )}
                </form.Field>

                <form.Field name="validityDays">
                  {(field) => (
                    <div className="space-y-2">
                      <Label htmlFor={field.name}>
                        {m['points.plan_config_form_validity_days_label']()}
                      </Label>
                      <Input
                        id={field.name}
                        type="number"
                        min="1"
                        value={field.state.value}
                        onChange={(e) => field.handleChange(parseInt(e.target.value) || 30)}
                        placeholder="30"
                        data-testid="validity-days"
                      />
                      <p className="text-xs text-muted-foreground">
                        {m['points.plan_config_form_validity_days_help']()}
                      </p>
                    </div>
                  )}
                </form.Field>
              </div>

              {/* Max Periods */}
              <form.Field name="maxPeriods">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor={field.name}>
                      {m['points.plan_config_form_max_periods_label']()}
                      <Badge variant="outline" className="ml-2">
                        {m['points.plan_config_form_max_periods_badge']()}
                      </Badge>
                    </Label>
                    <Input
                      id={field.name}
                      type="number"
                      min="0"
                      value={field.state.value ?? ''}
                      onChange={(e) =>
                        field.handleChange(e.target.value ? parseInt(e.target.value) : null)
                      }
                      placeholder="12"
                      data-testid="max-periods"
                    />
                    <p className="text-xs text-muted-foreground">
                      {m['points.plan_config_form_max_periods_help']()}
                    </p>
                  </div>
                )}
              </form.Field>

              {/* Action Buttons */}
              <div className="flex justify-end gap-3 pt-4">
                <Button
                  type="button"
                  variant="outline"
                  onClick={onCancel}
                  disabled={isSubmitting}
                  data-testid="cancel-button"
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={isSubmitting} data-testid="submit-button">
                  {isSubmitting
                    ? m['shared.saving']()
                    : isEdit
                      ? m['shared.save_changes']()
                      : m['shared.create_configuration']()}
                </Button>
              </div>
            </div>
          </AppForm>
        </form>
      </CardContent>
    </Card>
  )
}
