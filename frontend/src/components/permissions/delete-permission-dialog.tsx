import { ConfirmDialog } from '@/components/shared'
import { deletePermission } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { useRealmId } from '@/stores/auth-store'
import type { PermissionResponse } from '@/lib/api-generated'
import { queryKeys } from '@/data/query-options'

interface DeletePermissionDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  permission: PermissionResponse
  realmId?: string // Optional for backward compatibility
}

export function DeletePermissionDialog({
  open,
  onOpenChange,
  permission,
  realmId: realmIdProp,
}: DeletePermissionDialogProps) {
  const storeRealmId = useRealmId()
  const realmId = realmIdProp ?? storeRealmId
  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (permissionId: string) =>
      deletePermission({
        path: { realmId, permissionDefinitionId: permissionId },
      }),
    getSuccessMessage: () => `Permission "${permission.name}" deleted successfully`,
    invalidateQueries: [queryKeys.permissions(realmId)],
    onSuccess: () => {
      onOpenChange(false)
    },
  })

  const handleDelete = () => {
    mutate(permission.id)
  }

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Delete Permission"
      description={`Are you sure you want to delete the permission "${permission.name}"? This action cannot be undone.`}
      onConfirm={handleDelete}
      isPending={isSubmitting}
      confirmTestId="permission-delete-confirm-button"
    />
  )
}
