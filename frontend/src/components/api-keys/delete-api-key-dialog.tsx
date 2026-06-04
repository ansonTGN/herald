import { ConfirmDialog } from '@/components/shared'
import { m } from '@/paraglide/messages'

interface DeleteApiKeyDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
  apiKeyName: string
}

export function DeleteApiKeyDialog({
  open,
  onOpenChange,
  onConfirm,
  apiKeyName,
}: DeleteApiKeyDialogProps) {
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={m['api_keys.delete_title']()}
      description={m['api_keys.delete_description']({ name: apiKeyName })}
      onConfirm={onConfirm}
      contentTestId="delete-confirmation-dialog"
      cancelTestId="cancel-delete-button"
      confirmTestId="confirm-delete-button"
    />
  )
}
