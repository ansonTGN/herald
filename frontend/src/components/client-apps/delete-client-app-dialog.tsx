import { ConfirmDialog } from '@/components/shared'
import { m } from '@/paraglide/messages'

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
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={m['client_apps.delete_title']()}
      description={<>{m['client_apps.delete_description']({ name: clientAppName })}</>}
      onConfirm={onConfirm}
      contentTestId="delete-confirmation-dialog"
      cancelTestId="cancel-delete-button"
      confirmTestId="confirm-delete-button"
    />
  )
}
