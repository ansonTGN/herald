import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Plus } from 'lucide-react'
import {
  type ProductResponse,
  createProduct,
  updateProduct,
  deleteProduct,
} from '@/lib/api-generated'
import { productsQueryOptions, queryKeys } from '@/data/query-options'
import { ProductTable } from './product-table'
import { ProductFormDialog } from './product-form-dialog'
import { ProductDeleteDialog } from './product-delete-dialog'
import { toast } from 'sonner'
import { type ProductFormData } from '@/lib/schemas/billing-forms'
import { PageHeader } from '@/components/shared/page-header'
import { handleApiResponse } from '@/lib/api-utils'
import { m } from '@/paraglide/messages'

interface ProductPageProps {
  realmId: string
}

export function ProductPage({ realmId }: ProductPageProps) {
  const queryClient = useQueryClient()
  const { data: products, isLoading, error } = useQuery(productsQueryOptions(realmId))

  const [productFormOpen, setProductFormOpen] = useState(false)
  const [editingProduct, setEditingProduct] = useState<ProductResponse | undefined>(undefined)
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false)
  const [deletingProduct, setDeletingProduct] = useState<ProductResponse | undefined>(undefined)

  const createUpdateProductMutation = useMutation({
    mutationFn: async (data: {
      formData: ProductFormData
      editingProductId?: string
    }): Promise<ProductResponse> => {
      const { formData, editingProductId } = data
      if (editingProductId) {
        return handleApiResponse(
          await updateProduct({
            path: { realmId, productId: editingProductId },
            body: {
              title: formData.title,
              description: formData.description ?? null,
              enabled: formData.enabled,
            },
          })
        )
      } else {
        return handleApiResponse(
          await createProduct({
            path: { realmId },
            body: formData,
          })
        )
      }
    },
    onSuccess: (data: ProductResponse, variables) => {
      const action = variables.editingProductId ? m['billing.updated']() : m['billing.created']()
      toast.success(m['billing.product_saved']({ title: data?.title, action }))
      setProductFormOpen(false)
      setEditingProduct(undefined)
      queryClient.invalidateQueries({ queryKey: queryKeys.billingProducts(realmId) })
      queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
    },
    onError: (error: Error) => {
      toast.error(m['billing.product_save_failed']({ message: error.message }))
    },
  })

  const deleteProductMutation = useMutation({
    mutationFn: async (data: { productId: string; title: string }) => {
      const response = await deleteProduct({ path: { realmId, productId: data.productId } })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (_, variables) => {
      toast.success(m['billing.product_deleted']({ title: variables.title }))
      setDeleteConfirmOpen(false)
      setDeletingProduct(undefined)
      queryClient.invalidateQueries({ queryKey: queryKeys.billingProducts(realmId) })
      queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
    },
    onError: (error: Error) => {
      toast.error(m['billing.product_delete_failed']({ message: error.message }))
    },
  })

  function handleCreateProduct() {
    setEditingProduct(undefined)
    setProductFormOpen(true)
  }

  function handleEditProduct(product: ProductResponse) {
    setEditingProduct(product)
    setProductFormOpen(true)
  }

  function handleDeleteProduct(product: ProductResponse) {
    setDeletingProduct(product)
    setDeleteConfirmOpen(true)
  }

  async function confirmDeleteProduct() {
    if (!deletingProduct) return
    await deleteProductMutation.mutateAsync({
      productId: deletingProduct.id,
      title: deletingProduct.title,
    })
  }

  async function handleProductSubmit(formData: ProductFormData) {
    await createUpdateProductMutation.mutateAsync({
      formData,
      editingProductId: editingProduct?.id,
    })
  }

  return (
    <div className="space-y-6" data-testid="products-page">
      <PageHeader
        title={m['billing.products_title']()}
        action={{
          label: m['billing.create_product'](),
          onClick: handleCreateProduct,
          testId: 'add-product-button',
          icon: <Plus className="mr-2 h-4 w-4" />,
        }}
      />

      <Card>
        <CardHeader>
          <CardTitle>{m['billing.products_title']()}</CardTitle>
        </CardHeader>
        <CardContent>
          <ProductTable
            data={products}
            isLoading={isLoading}
            error={error ?? undefined}
            onEdit={handleEditProduct}
            onDelete={handleDeleteProduct}
          />
        </CardContent>
      </Card>

      <ProductFormDialog
        product={editingProduct}
        open={productFormOpen}
        onOpenChange={setProductFormOpen}
        onSubmit={handleProductSubmit}
        isSubmitting={createUpdateProductMutation.isPending}
      />

      {deletingProduct && (
        <ProductDeleteDialog
          product={deletingProduct}
          open={deleteConfirmOpen}
          onOpenChange={setDeleteConfirmOpen}
          onConfirm={confirmDeleteProduct}
          isDeleting={deleteProductMutation.isPending}
          hasPlans={deletingProduct.plansCount > 0}
        />
      )}
    </div>
  )
}
