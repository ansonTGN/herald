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
import { Info } from 'lucide-react'
import type { PointsPlanConfigResponse } from '@/lib/api-generated'
import type { PointsPlanConfigFormData } from '@/lib/schemas/points-forms'
import { pointsPlanConfigSchema } from '@/lib/schemas/points-forms'

interface PointsPlanConfigFormProps {
  config?: PointsPlanConfigResponse | null
  plans: Array<{ id: string; name: string; title: string }>
  onSubmit: (data: PointsPlanConfigFormData) => void
  onCancel: () => void
  isSubmitting?: boolean
}

export function PointsPlanConfigForm({
  config,
  plans,
  onSubmit,
  onCancel,
  isSubmitting = false,
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
      <CardHeader>
        <CardTitle>
          {isEdit ? 'Edit Points Plan Configuration' : 'Create Points Plan Configuration'}
        </CardTitle>
      </CardHeader>
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
                    <Label htmlFor={field.name}>Plan *</Label>
                    <Select
                      value={field.state.value}
                      onValueChange={field.handleChange}
                      disabled={isEdit}
                      data-testid="plan-select"
                    >
                      <SelectTrigger id={field.name}>
                        <SelectValue placeholder="Select a plan" />
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
                        Plan cannot be changed after configuration is created
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
                      Points per Period *
                      <Badge variant="outline" className="ml-2">
                        <Info className="h-3 w-3 mr-1" />
                        Bonus
                      </Badge>
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
                      Points awarded for each period (subscribe or renewal)
                    </p>
                  </div>
                )}
              </form.Field>

              {/* Grant Settings */}
              <div className="space-y-4 p-4 bg-muted/50 rounded-lg">
                <div className="flex items-center justify-between">
                  <div className="space-y-1">
                    <Label htmlFor="grant-on-subscribe">Grant on Subscribe</Label>
                    <p className="text-xs text-muted-foreground">
                      Award points when user subscribes to this plan
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
                      <Label htmlFor={field.name}>Grant Period</Label>
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
                          <SelectItem value="once">Once</SelectItem>
                          <SelectItem value="daily">Daily</SelectItem>
                          <SelectItem value="weekly">Weekly</SelectItem>
                          <SelectItem value="monthly">Monthly</SelectItem>
                        </SelectContent>
                      </Select>
                      <p className="text-xs text-muted-foreground">
                        How often points are awarded (on subscription and/or renewal)
                      </p>
                    </div>
                  )}
                </form.Field>

                <form.Field name="validityDays">
                  {(field) => (
                    <div className="space-y-2">
                      <Label htmlFor={field.name}>Validity Days *</Label>
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
                        Number of days awarded points remain valid
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
                      Maximum Periods (Optional)
                      <Badge variant="outline" className="ml-2">
                        Limit
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
                      Maximum number of periods to award points. Leave empty for unlimited.
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
                    ? 'Saving...'
                    : isEdit
                      ? 'Update Configuration'
                      : 'Create Configuration'}
                </Button>
              </div>
            </div>
          </AppForm>
        </form>
      </CardContent>
    </Card>
  )
}
