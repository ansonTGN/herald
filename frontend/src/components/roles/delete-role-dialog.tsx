import { ConfirmDeleteDialog } from '@/components/shared'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { deleteRole } from '@/lib/api-generated'
import { useRealmId } from '@/stores/auth-store'
import type { RoleResponse } from '@/lib/api-generated'
import { queryKeys } from '@/data/query-options'

interface DeleteRoleDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  role: RoleResponse
  realmId?: string // Optional for backward compatibility
}

export function DeleteRoleDialog({
  open,
  onOpenChange,
  role,
  realmId: realmIdProp,
}: DeleteRoleDialogProps) {
  const storeRealmId = useRealmId()
  const realmId = realmIdProp ?? storeRealmId
  const { isSubmitting, mutate } = useFormMutation({
    mutationFn: (_: void) =>
      deleteRole({
        path: { realmId, roleId: role.id },
      }),
    getSuccessMessage: () => `Role "${role.name}" deleted successfully`,
    invalidateQueries: [queryKeys.roles(realmId)],
    onSuccess: () => {
      onOpenChange(false)
    },
  })

  const handleDelete = () => {
    mutate()
  }

  return (
    <ConfirmDeleteDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Delete Role"
      description={
        <>
          Are you sure you want to delete the role <strong>"{role.name}"</strong>? This action
          cannot be undone.
        </>
      }
      onConfirm={handleDelete}
      isPending={isSubmitting}
      confirmTestId="role-delete-confirm-button"
    />
  )
}
