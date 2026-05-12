import { useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Shield, Loader2 } from 'lucide-react'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { assignPermissionToRole, removePermissionFromRole } from '@/lib/api-generated'
import { PermissionCheckboxList } from './permission-checkbox-list'
import { useRealmId } from '@/stores/auth-store'
import type { RoleResponse, PermissionResponse } from '@/lib/api-generated'
import { queryKeys } from '@/data/query-options'

interface RolePermissionsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  role: RoleResponse
  realmId?: string // Optional for backward compatibility
  allPermissions: PermissionResponse[]
  assignedPermissionIds: string[]
}

export function RolePermissionsDialog({
  open,
  onOpenChange,
  role,
  realmId: realmIdProp,
  allPermissions,
  assignedPermissionIds,
}: RolePermissionsDialogProps) {
  const storeRealmId = useRealmId()
  const realmId = realmIdProp ?? storeRealmId
  const [optimisticAssignedIds, setOptimisticAssignedIds] =
    useState<string[]>(assignedPermissionIds)

  // Reset optimistic state when dialog opens or assigned permissions change
  if (open && optimisticAssignedIds.join(',') !== assignedPermissionIds.join(',')) {
    setOptimisticAssignedIds(assignedPermissionIds)
  }

  // Assign permission mutation
  const { isSubmitting: isAssigning, mutate: assignPermission } = useFormMutation({
    mutationFn: async (permissionId: string) => {
      const response = await assignPermissionToRole({
        path: { realmId, roleId: role.id },
        body: { permissionId },
      })

      if (response.error) {
        throw response.error
      }

      return response.data
    },
    getSuccessMessage: () => `Permission assigned to role "${role.name}"`,
    invalidateQueries: [queryKeys.rolePermissions(realmId, role.id)],
    onSuccess: () => {
      // Refresh will happen automatically via invalidateQueries
    },
  })

  // Remove permission mutation
  const { isSubmitting: isRemoving, mutate: removePermission } = useFormMutation({
    mutationFn: async (permissionId: string) => {
      const response = await removePermissionFromRole({
        path: { realmId, roleId: role.id, permissionId },
      })

      if (response.error) {
        throw response.error
      }

      return response.data
    },
    getSuccessMessage: () => `Permission removed from role "${role.name}"`,
    invalidateQueries: [queryKeys.rolePermissions(realmId, role.id)],
    onSuccess: () => {
      // Refresh will happen automatically via invalidateQueries
    },
  })

  const handleTogglePermission = (permissionId: string, checked: boolean) => {
    // Optimistic update
    if (checked) {
      setOptimisticAssignedIds((prev) => [...prev, permissionId])
      assignPermission(permissionId)
    } else {
      setOptimisticAssignedIds((prev) => prev.filter((id) => id !== permissionId))
      removePermission(permissionId)
    }
  }

  const isLoading = isAssigning || isRemoving

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[80vh]" data-testid="role-permissions-dialog">
        <DialogHeader>
          <div className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            <DialogTitle>Manage Permissions for {role.name}</DialogTitle>
          </div>
          <DialogDescription>
            Assign or remove permissions for this role. Permissions are grouped by resource.
            {role.isBuiltin && (
              <span className="block mt-2 text-amber-600 dark:text-amber-400 font-medium">
                Built-in permissions cannot be removed from built-in roles.
              </span>
            )}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* Summary stats */}
          <div className="flex items-center gap-4 text-sm">
            <div className="flex items-center gap-2">
              <Badge variant="outline">
                {optimisticAssignedIds.length} / {allPermissions.length}
              </Badge>
              <span className="text-muted-foreground">permissions assigned</span>
            </div>
            {isLoading && (
              <div className="flex items-center gap-2 text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>Saving changes...</span>
              </div>
            )}
          </div>

          {/* Permission list */}
          <PermissionCheckboxList
            permissions={allPermissions}
            assignedPermissionIds={optimisticAssignedIds}
            onTogglePermission={handleTogglePermission}
            isBuiltinRole={role.isBuiltin}
            disabled={isLoading}
            data-testid="role-permissions-checkbox-list"
          />
        </div>

        {/* Footer actions */}
        <div className="flex justify-end pt-4 border-t">
          <Button
            onClick={() => onOpenChange(false)}
            disabled={isLoading}
            data-testid="role-permissions-close-button"
          >
            Close
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
