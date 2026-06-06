import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Settings } from 'lucide-react'
import { pointsPlanConfigsQueryOptions, queryKeys } from '@/data/query-options'
import { toast } from 'sonner'
import { ConfirmDialog, PageHeader } from '@/components/shared'
import { m } from '@/paraglide/messages'
import type { LocalPointsPlanConfig } from '@/types/points-plan-config'

interface PointsConfigsPageProps {
  realmId: string
}

export function PointsConfigsPage({ realmId }: PointsConfigsPageProps) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  // UI state
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false)
  const [deletingConfig, setDeletingConfig] = useState<LocalPointsPlanConfig | null>(null)

  // Queries
  const { data: configs, isLoading: configsLoading } = useQuery(
    pointsPlanConfigsQueryOptions(realmId)
  )

  // Mutations -- TODO: deletePlanConfig API was removed, stub until migration
  const deleteConfigMutation = useMutation({
    mutationFn: async (_configId: string) => {
      // TODO: implement delete with new entitlement-based API
      throw new Error('Not implemented: points config delete is pending migration')
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

  function handleEditConfig(config: LocalPointsPlanConfig) {
    navigate({
      to: '/$realmId/manage/points/configs/$configId/edit',
      params: { realmId, configId: config.configId },
    })
  }

  function handleDeleteConfig(config: LocalPointsPlanConfig) {
    setDeletingConfig(config)
    setDeleteConfirmOpen(true)
  }

  async function confirmDeleteConfig() {
    if (!deletingConfig) return
    await deleteConfigMutation.mutateAsync(deletingConfig.configId)
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

      {configsLoading ? (
        <div className="text-center py-8">{m['points.configs_loading']()}</div>
      ) : configs && configs.length > 0 ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {configs.map((config) => (
            <Card key={config.configId} data-testid={`config-card-${config.configId}`}>
              <CardContent className="space-y-3 pt-6">
                <div className="flex justify-between">
                  <span className="text-sm text-muted-foreground">
                    {m['points.config_card_points_per_period']()}
                  </span>
                  <span className="font-semibold">+{config.pointsPerPeriod.toLocaleString()}</span>
                </div>
                <div className="flex gap-2 pt-2">
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
                    disabled
                    title="Not yet available"
                    data-testid={`delete-config-${config.configId}`}
                  >
                    {m['common.delete']()}
                  </Button>
                  <span className="text-xs text-muted-foreground self-center">N/A</span>
                </div>
              </CardContent>
            </Card>
          ))}
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
        description={m['points.configs_delete_description']({
          planName: deletingConfig ? 'this configuration' : '',
        })}
        onConfirm={confirmDeleteConfig}
        isPending={deleteConfigMutation.isPending}
        confirmTestId="confirm-delete-config"
      />
    </div>
  )
}
