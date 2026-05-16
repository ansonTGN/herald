import { useState } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Card, CardContent } from '@/components/ui/card'
import { Plus } from 'lucide-react'
import { type PointsPackageResponse, deletePointsPackage } from '@/lib/api-generated'
import { pointsPackagesQueryOptions, queryKeys } from '@/data/query-options'
import { PointsPackageList } from '@/components/points-packages/points-package-list'
import { PointsPackageDeleteDialog } from '@/components/points-packages/points-package-delete-dialog'
import { PaymentProviderConfigForm } from '@/components/points-packages/payment-provider-config-form'
import { toast } from 'sonner'
import { PageHeader } from '@/components/shared/page-header'

export const Route = createFileRoute('/$realmId/manage/points-packages/')({
  component: PointsPackagesPage,
})

function PointsPackagesPage() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const { data: packages, isLoading, error } = useQuery(pointsPackagesQueryOptions(realmId))

  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false)
  const [deletingPackage, setDeletingPackage] = useState<PointsPackageResponse | undefined>(
    undefined
  )
  const [configuringPackage, setConfiguringPackage] = useState<PointsPackageResponse | undefined>(
    undefined
  )

  const deletePackageMutation = useMutation({
    mutationFn: async (data: { packageId: string; title: string }) => {
      const response = await deletePointsPackage({ path: { realmId, packageId: data.packageId } })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (_, variables) => {
      toast.success(`Points package "${variables.title}" deleted successfully`)
      setDeleteConfirmOpen(false)
      setDeletingPackage(undefined)
      queryClient.invalidateQueries({ queryKey: queryKeys.pointsPackages(realmId) })
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete package: ${error.message}`)
    },
  })

  function handleEditPackage(pkg: PointsPackageResponse) {
    navigate({
      to: '/$realmId/manage/points-packages/$packageId/edit',
      params: { realmId, packageId: pkg.id },
    })
  }

  function handleDeletePackage(pkg: PointsPackageResponse) {
    setDeletingPackage(pkg)
    setDeleteConfirmOpen(true)
  }

  function handleConfigureProviders(pkg: PointsPackageResponse) {
    setConfiguringPackage(pkg)
  }

  async function confirmDeletePackage() {
    if (!deletingPackage) return
    await deletePackageMutation.mutateAsync({
      packageId: deletingPackage.id,
      title: deletingPackage.title,
    })
  }

  return (
    <div className="space-y-6" data-testid="points-packages-page">
      <PageHeader
        title="Points Packages"
        action={{
          label: 'Create Package',
          onClick: () =>
            navigate({
              to: '/$realmId/manage/points-packages/new',
              params: { realmId },
            }),
          testId: 'add-points-package-button',
          icon: <Plus className="mr-2 h-4 w-4" />,
        }}
      />

      <Card>
        <CardContent>
          <PointsPackageList
            data={packages || []}
            isLoading={isLoading}
            error={error || undefined}
            onEdit={handleEditPackage}
            onDelete={handleDeletePackage}
            onConfigureProviders={handleConfigureProviders}
          />
        </CardContent>
      </Card>

      {deletingPackage && (
        <PointsPackageDeleteDialog
          package={deletingPackage}
          open={deleteConfirmOpen}
          onOpenChange={setDeleteConfirmOpen}
          onConfirm={confirmDeletePackage}
          isDeleting={deletePackageMutation.isPending}
          hasPurchaseHistory={false} // TODO: Check actual purchase history
        />
      )}

      {configuringPackage && (
        <PaymentProviderConfigForm
          packageId={configuringPackage.id}
          realmId={realmId}
          open={!!configuringPackage}
          onOpenChange={(open) => !open && setConfiguringPackage(undefined)}
        />
      )}
    </div>
  )
}
