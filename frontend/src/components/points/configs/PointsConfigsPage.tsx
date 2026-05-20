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
import { ConfirmDeleteDialog, PageHeader } from '@/components/shared'

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
      toast.success('Points plan configuration deleted successfully')
      setDeleteConfirmOpen(false)
      setDeletingConfig(null)
      queryClient.invalidateQueries({ queryKey: queryKeys.pointsPlanConfigs(realmId) })
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete configuration: ${error.message}`)
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
        title="Points Rules"
        action={{
          label: 'Create Points Rule',
          onClick: handleCreateConfig,
          testId: 'create-config-button',
        }}
      />

      {configs && configs.length > 0 && (
        <div className="flex justify-end gap-2">
          <ExportGuideButton
            configs={configs.map((c) => ({
              planName: plansMap.get(c.planId)?.title || plansMap.get(c.planId)?.name || 'Unknown',
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
            View All Guides
          </Button>
        </div>
      )}

      {configsLoading ? (
        <div className="text-center py-8">Loading configurations...</div>
      ) : configs && configs.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {configs.map((config) => {
            const plan = plansMap.get(config.planId)
            return (
              <Card key={config.configId} data-testid={`config-card-${config.configId}`}>
                <CardHeader>
                  <CardTitle className="text-lg">
                    {plan?.title || plan?.name || 'Unknown Plan'}
                  </CardTitle>
                  <Badge variant={config.active ? 'default' : 'secondary'}>
                    {config.active ? 'Active' : 'Inactive'}
                  </Badge>
                </CardHeader>
                <CardContent className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Points per Period</span>
                    <span className="font-semibold">
                      +{config.pointsPerPeriod.toLocaleString()}
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Grant Period</span>
                    <span className="font-semibold">{config.grantPeriodType}</span>
                  </div>
                  {config.grantOnSubscribe && (
                    <div className="flex justify-between">
                      <span className="text-sm text-muted-foreground">Grant on Subscribe</span>
                      <span className="font-semibold text-green-600">Yes</span>
                    </div>
                  )}
                  <div className="flex justify-between">
                    <span className="text-sm text-muted-foreground">Validity Days</span>
                    <span className="font-semibold">{config.validityDays}</span>
                  </div>
                  {config.maxPeriods && (
                    <div className="flex justify-between">
                      <span className="text-sm text-muted-foreground">Max Periods</span>
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
                      View
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleShareGuide(config)}
                      data-testid={`points-share-guide-${config.configId}`}
                    >
                      <Share2 className="h-3 w-3 mr-1" />
                      Share
                    </Button>
                    <div className="flex-1" />
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleEditConfig(config)}
                      data-testid={`edit-config-${config.configId}`}
                    >
                      Edit
                    </Button>
                    <Button
                      variant="destructive"
                      size="sm"
                      onClick={() => handleDeleteConfig(config)}
                      data-testid={`delete-config-${config.configId}`}
                    >
                      Delete
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
            <p className="text-muted-foreground">No points plan configurations found</p>
          </CardContent>
        </Card>
      )}

      {/* Delete Confirmation Dialog */}
      <ConfirmDeleteDialog
        open={deleteConfirmOpen}
        onOpenChange={setDeleteConfirmOpen}
        title="Delete Points Rule"
        description={
          <>
            Are you sure you want to delete the points configuration for{' '}
            {deletingConfig && plansMap.get(deletingConfig.planId)?.title}? This action cannot be
            undone.
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
          planName={plansMap.get(selectedGuideConfig.planId)?.title || 'Unknown Plan'}
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
