import { ConfirmDialog } from '@/components/shared'
import type { OAuthConfigResponse } from '@/lib/api-generated'

interface DeleteProviderDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  provider: OAuthConfigResponse
  onConfirm: () => void
  isPending?: boolean
}

export function DeleteProviderDialog({
  open,
  onOpenChange,
  provider,
  onConfirm,
  isPending = false,
}: DeleteProviderDialogProps) {
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Delete Provider Configuration"
      description={`Are you sure you want to delete the ${provider.providerType} provider? This action cannot be undone and users will no longer be able to use this provider for login.`}
      onConfirm={onConfirm}
      isPending={isPending}
      confirmTestId="provider-delete-confirm-button"
    />
  )
}
