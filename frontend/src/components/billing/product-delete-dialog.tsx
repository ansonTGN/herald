import { ConfirmDeleteDialog } from '@/components/shared'

interface ProductDeleteDialogProps {
  product: {
    title: string
  }
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
  isDeleting: boolean
  hasPlans: boolean
}

export function ProductDeleteDialog({
  product,
  open,
  onOpenChange,
  onConfirm,
  isDeleting,
  hasPlans,
}: ProductDeleteDialogProps) {
  return (
    <ConfirmDeleteDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Delete Product"
      description={
        hasPlans
          ? 'This product cannot be deleted because it has associated plans. Please move or delete all plans first.'
          : `Are you sure you want to delete product "${product.title}"?`
      }
      onConfirm={onConfirm}
      confirmDisabled={hasPlans || isDeleting}
      isPending={isDeleting}
      contentTestId="product-delete-confirm-dialog"
      cancelTestId="product-delete-cancel-button"
      confirmTestId="product-delete-confirm-button"
    />
  )
}
