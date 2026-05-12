import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Plus } from 'lucide-react'
import {
  type PointsPackageResponse,
  createPointsPackage,
  updatePointsPackage,
  deletePointsPackage,
} from '@/lib/api-generated'
import { pointsPackagesQueryOptions, queryKeys } from '@/data/query-options'
import { PointsPackageList } from '@/components/points-packages/points-package-list'
import { PointsPackageFormDialog } from '@/components/points-packages/points-package-form-dialog'
import { PointsPackageDeleteDialog } from '@/components/points-packages/points-package-delete-dialog'
import { PaymentProviderConfigForm } from '@/components/points-packages/payment-provider-config-form'
import { toast } from 'sonner'
import {
  displayPriceToApiPrice,
  type PointsPackageFormData,
} from '@/lib/schemas/points-package-forms'
import { PageHeader } from '@/components/shared/page-header'
import { Button } from '@/components/ui/button'

export const Route = createFileRoute('/$realmId/manage/points-packages')({
  component: PointsPackagesPage,
})

function PointsPackagesPage() {
  const { realmId } = Route.useParams()
  const queryClient = useQueryClient()
  const { data: packages, isLoading, error } = useQuery(pointsPackagesQueryOptions(realmId))

  const [packageFormOpen, setPackageFormOpen] = useState(false)
  const [editingPackage, setEditingPackage] = useState<PointsPackageResponse | undefined>(undefined)
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false)
  const [deletingPackage, setDeletingPackage] = useState<PointsPackageResponse | undefined>(
    undefined
  )
  const [configuringPackage, setConfiguringPackage] = useState<PointsPackageResponse | undefined>(
    undefined
  )

  const createUpdatePackageMutation = useMutation({
    mutationFn: async (data: {
      formData: PointsPackageFormData
      editingPackageId?: string
    }): Promise<PointsPackageResponse> => {
      const { formData, editingPackageId } = data

      // KEY FIX: Convert display price to API price (cents)
      const apiPrice = displayPriceToApiPrice(formData.price, formData.currency)

      if (editingPackageId) {
        const response = await updatePointsPackage({
          path: { realmId, packageId: editingPackageId },
          body: {
            // Don't include immutable 'points' field in update
            title: formData.title,
            description: formData.description ?? null,
            price: apiPrice, // Send integer cents
            currency: formData.currency,
            sortOrder: formData.sortOrder,
            enabled: formData.enabled,
          },
        })
        if (response.error) throw response.error
        if (!response.data) throw new Error('Failed to update package')
        return response.data as unknown as PointsPackageResponse
      } else {
        const response = await createPointsPackage({
          path: { realmId },
          body: {
            ...formData,
            price: apiPrice, // Send integer cents
          },
        })
        if (response.error) throw response.error
        if (!response.data) throw new Error('Failed to create package')
        return response.data as unknown as PointsPackageResponse
      }
    },
    onSuccess: (data: PointsPackageResponse, variables) => {
      const action = variables.editingPackageId ? 'updated' : 'created'
      toast.success(`Points package "${data?.title}" ${action} successfully`)
      setPackageFormOpen(false)
      setEditingPackage(undefined)
      queryClient.invalidateQueries({ queryKey: queryKeys.pointsPackages(realmId) })
    },
    onError: (error: Error) => {
      toast.error(`Failed to save package: ${error.message}`)
    },
  })

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

  function handleCreatePackage() {
    setEditingPackage(undefined)
    setPackageFormOpen(true)
  }

  function handleEditPackage(pkg: PointsPackageResponse) {
    setEditingPackage(pkg)
    setPackageFormOpen(true)
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
    <div className="container" data-testid="points-packages-page">
      <PageHeader
        title="Points Packages"
        description="Manage points packages that users can purchase"
      />

      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>Points Packages</CardTitle>
            <Button onClick={handleCreatePackage} data-testid="add-points-package-button">
              <Plus className="mr-2 h-4 w-4" />
              Create Package
            </Button>
          </div>
        </CardHeader>
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

      <PointsPackageFormDialog
        package={editingPackage}
        open={packageFormOpen}
        onOpenChange={setPackageFormOpen}
        onSubmit={async (formData) => {
          await createUpdatePackageMutation.mutateAsync({
            formData,
            editingPackageId: editingPackage?.id,
          })
        }}
        isSubmitting={createUpdatePackageMutation.isPending}
      />

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
