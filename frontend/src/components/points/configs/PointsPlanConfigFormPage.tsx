import { useMemo } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { ArrowLeft } from 'lucide-react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { PointsPlanConfigForm } from '../PointsPlanConfigForm'
import { createPlanConfig, updatePlanConfig } from '@/lib/api-generated'
import type { PointsPlanConfigResponse, SubscriptionPlanResponse } from '@/lib/api-generated'
import type { PointsPlanConfigFormData } from '@/lib/schemas/points-forms'
import { queryKeys } from '@/data/query-options'
import { m } from '@/paraglide/messages'

interface PointsPlanConfigFormPageProps {
  mode: 'create' | 'edit'
  realmId: string
  config?: PointsPlanConfigResponse
  plans: SubscriptionPlanResponse[]
}

export function PointsPlanConfigFormPage({
  mode,
  realmId,
  config,
  plans,
}: PointsPlanConfigFormPageProps) {
  const isEditing = mode === 'edit'
  const navigate = useNavigate()
  const queryClient = useQueryClient()

  const handleBack = () => {
    navigate({ to: '/$realmId/manage/points/configs', params: { realmId } })
  }

  const formPlans = useMemo(
    () =>
      plans.map((plan) => ({
        id: plan.id,
        name: plan.name,
        title: plan.title,
      })),
    [plans]
  )

  const saveMutation = useMutation({
    mutationFn: async (data: PointsPlanConfigFormData) => {
      if (isEditing && config) {
        const response = await updatePlanConfig({
          path: { realmId, configId: config.configId },
          body: {
            points_per_period: data.pointsPerPeriod,
            grant_on_subscribe: data.grantOnSubscribe,
            grant_period_type: data.grantPeriodType,
            max_periods: data.maxPeriods,
            validity_days: data.validityDays,
          },
        })
        if (response.error) throw response.error
        if (!response.data) throw new Error('Failed to update points rule')
        return response.data
      }

      const response = await createPlanConfig({
        path: { realmId },
        body: {
          planId: data.planId,
          pointsPerPeriod: data.pointsPerPeriod,
          grantOnSubscribe: data.grantOnSubscribe,
          grantPeriodType: data.grantPeriodType,
          maxPeriods: data.maxPeriods,
          validityDays: data.validityDays,
        },
      })
      if (response.error) throw response.error
      if (!response.data) throw new Error('Failed to create points rule')
      return response.data
    },
    onSuccess: async () => {
      toast.success(
        m['points.plan_config_saved_success']({
          action: isEditing
            ? m['common.update']().toLowerCase()
            : m['common.create']().toLowerCase(),
        })
      )
      await queryClient.invalidateQueries({ queryKey: queryKeys.pointsPlanConfigs(realmId) })
      handleBack()
    },
    onError: (error: Error) => {
      toast.error(m['points.plan_config_save_failed']({ message: error.message }))
    },
  })

  if (isEditing && !config) {
    return (
      <div className="container max-w-4xl mx-auto py-6 px-6 space-y-6">
        <div className="flex items-center gap-4">
          <Button type="button" variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <div>
            <h1 className="text-2xl font-bold">{m['points.plan_config_not_found_title']()}</h1>
            <p className="text-muted-foreground text-sm">
              {m['points.plan_config_not_found_description']()}
            </p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div
      className="container max-w-4xl mx-auto py-6 px-6 space-y-6"
      data-testid="points-plan-config-form-page"
    >
      <div className="flex items-center gap-4">
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={handleBack}
          data-testid="points-rule-form-back-button"
        >
          <ArrowLeft className="h-4 w-4" />
        </Button>
        <div>
          <h1 className="text-2xl font-bold" data-testid="page-title">
            {isEditing
              ? m['points.plan_config_edit_title']()
              : m['points.plan_config_create_title']()}
          </h1>
          <p className="text-muted-foreground text-sm">
            {isEditing
              ? m['points.plan_config_edit_description']()
              : m['points.plan_config_create_description']()}
          </p>
        </div>
      </div>

      <PointsPlanConfigForm
        config={config ?? null}
        plans={formPlans}
        onSubmit={(data) => saveMutation.mutate(data)}
        onCancel={handleBack}
        isSubmitting={saveMutation.isPending}
        showTitle={false}
      />
    </div>
  )
}
