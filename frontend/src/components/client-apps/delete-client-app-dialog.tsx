import { ConfirmDeleteDialog } from '@/components/shared'

interface DeleteClientAppDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
  clientAppName: string
}

export function DeleteClientAppDialog({
  open,
  onOpenChange,
  onConfirm,
  clientAppName,
}: DeleteClientAppDialogProps) {
  return (
    <ConfirmDeleteDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Delete Client App"
      description={
        <>Are you sure you want to delete "{clientAppName}"? This action cannot be undone.</>
      }
      onConfirm={onConfirm}
      contentTestId="delete-confirmation-dialog"
      cancelTestId="cancel-delete-button"
      confirmTestId="confirm-delete-button"
    />
  )
}
