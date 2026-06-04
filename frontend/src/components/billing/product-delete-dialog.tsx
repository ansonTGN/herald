import { ConfirmDialog } from '@/components/shared'
import { m } from '@/paraglide/messages'

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
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={m['billing.delete_product_title']()}
      description={
        hasPlans
          ? m['billing.delete_product_has_plans']()
          : m['billing.delete_product_description']({ title: product.title })
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
