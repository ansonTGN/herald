import { ConfirmDialog } from '@/components/shared'
import type { OAuthConfigResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

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
      title={m['oauth.delete_title']()}
      description={m['oauth.delete_description']({ providerType: provider.providerType })}
      onConfirm={onConfirm}
      isPending={isPending}
      confirmTestId="provider-delete-confirm-button"
    />
  )
}
