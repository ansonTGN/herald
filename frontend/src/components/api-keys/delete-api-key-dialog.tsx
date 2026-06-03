import { ConfirmDialog } from '@/components/shared'

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
      title="Delete API Key"
      description={
        <>Are you sure you want to delete "{apiKeyName}"? This action cannot be undone.</>
      }
      onConfirm={onConfirm}
      contentTestId="delete-confirmation-dialog"
      cancelTestId="cancel-delete-button"
      confirmTestId="confirm-delete-button"
    />
  )
}
