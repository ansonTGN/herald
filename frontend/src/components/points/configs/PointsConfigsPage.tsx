import { useState, useMemo } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Settings, Eye, Share2 } from 'lucide-react'
import { PointsGuideDialog } from '../PointsGuideDialog'
import { ShareLinkDialog } from '../ShareLinkDialog'
import { ExportGuideButton } from '../ExportGuideButton'
import {
  pointsPlanConfigsQueryOptions,
  subscriptionPlansQueryOptions,
  queryKeys,
} from '@/data/query-options'
import { deletePlanConfig } from '@/lib/api-generated'
import type { PointsPlanConfigResponse } from '@/lib/api-generated'
import { toast } from 'sonner'
import { ConfirmDialog, PageHeader } from '@/components/shared'
import { m } from '@/paraglide/messages'

interface PointsConfigsPageProps {
  realmId: string
}

export function PointsConfigsPage({ realmId }: PointsConfigsPageProps) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  // UI state
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false)
  const [deletingConfig, setDeletingConfig] = useState<PointsPlanConfigResponse | null>(null)
  const [guideDialogOpen, setGuideDialogOpen] = useState(false)
  const [selectedGuideConfig, setSelectedGuideConfig] = useState<PointsPlanConfigResponse | null>(
    null
  )
  const [shareLinkDialogOpen, setShareLinkDialogOpen] = useState(false)

  // Queries
  const { data: configs, isLoading: configsLoading } = useQuery(
    pointsPlanConfigsQueryOptions(realmId)
  )

  const { data: plansData } = useQuery(subscriptionPlansQueryOptions(realmId))

  // Memoized plan map for O(1) lookup
  const plansMap = useMemo(() => {
    const plans = plansData?.items ?? []
    return new Map(plans?.map((plan) => [plan.id, plan]))
  }, [plansData?.items])

  // Mutations
  const deleteConfigMutation = useMutation({
    mutationFn: async (configId: string) => {
      const response = await deletePlanConfig({
        path: { realmId, configId },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success(m['points.configs_deleted_success']())
      setDeleteConfirmOpen(false)
      setDeletingConfig(null)
      queryClient.invalidateQueries({ queryKey: queryKeys.pointsPlanConfigs(realmId) })
    },
    onError: (error: Error) => {
      toast.error(m['points.configs_delete_failed']({ message: error.message }))
    },
  })

  // Handlers
  function handleCreateConfig() {
    navigate({ to: '/$realmId/manage/points/configs/new', params: { realmId } })
  }

  function handleEditConfig(config: PointsPlanConfigResponse) {
    navigate({
      to: '/$realmId/manage/points/configs/$configId/edit',
      params: { realmId, configId: config.configId },
    })
  }

  function handleDeleteConfig(config: PointsPlanConfigResponse) {
    setDeletingConfig(config)
    setDeleteConfirmOpen(true)
  }

  async function confirmDeleteConfig() {
    if (!deletingConfig) return
    await deleteConfigMutation.mutateAsync(deletingConfig.configId)
  }

  function handleViewGuide(config: PointsPlanConfigResponse) {
    setSelectedGuideConfig(config)
    setGuideDialogOpen(true)
  }

  function handleShareGuide(config: PointsPlanConfigResponse) {
    setSelectedGuideConfig(config)
    setShareLinkDialogOpen(true)
  }

  function getGuideUrl(config: PointsPlanConfigResponse): string {
    return `${window.location.origin}/${realmId}/points/guide?configId=${config.configId}`
  }

  return (
    <div className="space-y-6" data-testid="points-configs-page">
      <PageHeader
        title={m['points.configs_page_title']()}
        action={{
          label: m['points.configs_create_button'](),
          onClick: handleCreateConfig,
          testId: 'create-config-button',
        }}
      />

      {configs && configs.length > 0 && (
        <div className="flex justify-end gap-2">
          <ExportGuideButton
            configs={configs.map((c) => ({
              planName:
                plansMap.get(c.planId)?.title ||
                plansMap.get(c.planId)?.name ||
                m['points.config_card_unknown_plan'](),
              pointsPerPeriod: c.pointsPerPeriod,
              grantOnSubscribe: c.grantOnSubscribe,
              grantPeriodType: c.grantPeriodType,
              maxPeriods: c.maxPeriods,
              validityDays: c.validityDays,
            }))}
          />
          <Button
            variant="outline"
            onClick={() => setGuideDialogOpen(true)}
            data-testid="view-all-guides-button"
          >
            <Eye className="mr-2 h-4 w-4" />
            {m['points.configs_view_all_guides']()}
          </Button>
        </div>
      )}

      {configsLoading ? (
        <div className="text-center py-8">{m['points.configs_loading']()}</div>
      ) : configs && configs.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {configs.map((config) => {
            const plan = plansMap.get(config.planId)
            return (
              <Card key={config.configId} data-testid={`config-card-${config.configId}`}>
                <CardHeader>
                  <CardTitle className="text-lg">
                    {plan?.title || plan?.name || m['points.config_card_unknown_plan']()}
                  </CardTitle>
                  <Badge variant={config.active ? 'default' : 'secondary'}>
                    {config.active
                      ? m['points.config_card_active']()
                      : m['points.config_card_inactive']()}
                  </Badge>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">
                      {m['points.config_card_points_per_period']()}
                    </span>
                    <span className="font-semibold">
                      +{config.pointsPerPeriod.toLocaleString()}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">
                      {m['points.config_card_grant_period']()}
                    </span>
                    <span className="font-semibold">{config.grantPeriodType}</span>
                  </div>
                  {config.grantOnSubscribe && (
                    <div className="flex justify-between">
                      <span className="text-sm text-muted-foreground">
                        {m['points.config_card_grant_on_subscribe']()}
                      </span>
                      <span className="font-semibold text-green-600">{m['common.yes']()}</span>
                    </div>
                  )}
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">
                      {m['points.config_card_validity_days']()}
                    </span>
                    <span className="font-semibold">{config.validityDays}</span>
                  </div>
                  {config.maxPeriods && (
                    <div className="flex justify-between">
                      <span className="text-sm text-muted-foreground">
                        {m['points.config_card_max_periods']()}
                      </span>
                      <span className="font-semibold">{config.maxPeriods.toLocaleString()}</span>
                    </div>
                  )}
                  <div className="flex gap-2 pt-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleViewGuide(config)}
                      data-testid={`points-view-guide-${config.configId}`}
                    >
                      <Eye className="h-3 w-3 mr-1" />
                      {m['points.config_card_view']()}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleShareGuide(config)}
                      data-testid={`points-share-guide-${config.configId}`}
                    >
                      <Share2 className="h-3 w-3 mr-1" />
                      {m['points.config_card_share']()}
                    </Button>
                    <div className="flex-1" />
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleEditConfig(config)}
                      data-testid={`edit-config-${config.configId}`}
                    >
                      {m['common.edit']()}
                    </Button>
                    <Button
                      variant="destructive"
                      size="sm"
                      onClick={() => handleDeleteConfig(config)}
                      data-testid={`delete-config-${config.configId}`}
                    >
                      {m['common.delete']()}
                    </Button>
                  </div>
                </CardContent>
              </Card>
            )
          })}
        </div>
      ) : (
        <Card>
          <CardContent className="text-center py-12">
            <Settings className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
            <p className="text-muted-foreground">{m['points.configs_empty']()}</p>
          </CardContent>
        </Card>
      )}

      {/* Delete Confirmation Dialog */}
      <ConfirmDialog
        open={deleteConfirmOpen}
        onOpenChange={setDeleteConfirmOpen}
        title={m['points.configs_delete_title']()}
        description={
          <>
            {m['points.configs_delete_description']({
              planName: deletingConfig ? (plansMap.get(deletingConfig.planId)?.title ?? '') : '',
            })}
          </>
        }
        onConfirm={confirmDeleteConfig}
        isPending={deleteConfigMutation.isPending}
        confirmTestId="confirm-delete-config"
      />

      {/* Guide Dialog */}
      {selectedGuideConfig && (
        <PointsGuideDialog
          config={selectedGuideConfig}
          planName={
            plansMap.get(selectedGuideConfig.planId)?.title ||
            m['points.config_card_unknown_plan']()
          }
          open={guideDialogOpen}
          onClose={() => setGuideDialogOpen(false)}
        />
      )}

      {/* Share Link Dialog */}
      {selectedGuideConfig && (
        <ShareLinkDialog
          open={shareLinkDialogOpen}
          onClose={() => setShareLinkDialogOpen(false)}
          guideUrl={getGuideUrl(selectedGuideConfig)}
        />
      )}
    </div>
  )
}
