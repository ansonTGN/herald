import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { ArrowLeft } from 'lucide-react'
import { toast } from 'sonner'
import { Button } from '@/components/ui/button'
import { PointsPlanConfigForm } from '../PointsPlanConfigForm'
import type { PointsPlanConfigFormData } from '@/lib/schemas/points-forms'
import { queryKeys } from '@/data/query-options'
import { m } from '@/paraglide/messages'
import type { LocalPointsPlanConfig } from '@/types/points-plan-config'

interface PlanOption {
  id: string
  name: string
  title: string
}

interface PointsPlanConfigFormPageProps {
  mode: 'create' | 'edit'
  realmId: string
  config?: LocalPointsPlanConfig
  plans: PlanOption[]
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

  // TODO: createPlanConfig/updatePlanConfig APIs were removed by product_reduce.
  // These mutations will fail until points are migrated to entitlement-based config.
  const saveMutation = useMutation({
    mutationFn: async (_data: PointsPlanConfigFormData) => {
      throw new Error(
        'Not implemented: points config save is pending migration to entitlement-based config'
      )
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
        plans={plans}
        onSubmit={(data) => saveMutation.mutate(data)}
        onCancel={handleBack}
        isSubmitting={saveMutation.isPending}
        showTitle={false}
      />
    </div>
  )
}
