import { useState, useEffect } from 'react'
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
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { assignPermissionToRole, removePermissionFromRole } from '@/lib/api-generated'
import { PermissionCheckboxList } from './permission-checkbox-list'
import { useRealmId } from '@/stores/auth-store'
import type { RoleResponse, PermissionResponse } from '@/lib/api-generated'
import { queryKeys } from '@/data/query-options'

interface RolePermissionsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  role: RoleResponse
  realmId?: string
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
  const queryClient = useQueryClient()

  const [localAssignedIds, setLocalAssignedIds] = useState<string[]>(assignedPermissionIds)

  useEffect(() => {
    if (open) {
      setLocalAssignedIds(assignedPermissionIds)
    }
  }, [open, assignedPermissionIds])

  const addedIds = localAssignedIds.filter((id) => !assignedPermissionIds.includes(id))
  const removedIds = assignedPermissionIds.filter((id) => !localAssignedIds.includes(id))
  const hasChanges = addedIds.length > 0 || removedIds.length > 0

  const saveMutation = useMutation({
    mutationFn: async () => {
      const assignRequests = addedIds.map((permissionId) =>
        assignPermissionToRole({
          path: { realmId, roleId: role.id },
          body: { permissionId },
        }).then((response) => {
          if (response.error) throw response.error
          return response.data
        }),
      )

      const removeRequests = removedIds.map((permissionId) =>
        removePermissionFromRole({
          path: { realmId, roleId: role.id, permissionId },
        }).then((response) => {
          if (response.error) throw response.error
          return response.data
        }),
      )

      await Promise.all([...assignRequests, ...removeRequests])
      return { assigned: addedIds.length, removed: removedIds.length }
    },
    onSuccess: async (result) => {
      const parts: string[] = []
      if (result.assigned > 0) parts.push(`${result.assigned} permission(s) assigned`)
      if (result.removed > 0) parts.push(`${result.removed} permission(s) removed`)
      toast.success(`Permissions updated: ${parts.join(', ')}`)
      await queryClient.invalidateQueries({
        queryKey: queryKeys.rolePermissions(realmId, role.id),
      })
      onOpenChange(false)
    },
    onError: (error: Error) => {
      toast.error(`Failed to save permissions: ${error.message}`)
    },
  })

  const handleTogglePermission = (permissionId: string, checked: boolean) => {
    setLocalAssignedIds((prev) =>
      checked ? [...prev, permissionId] : prev.filter((id) => id !== permissionId),
    )
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="max-w-3xl max-h-[80vh] overflow-hidden flex flex-col"
        data-testid="role-permissions-dialog"
      >
        <DialogHeader className="flex-shrink-0">
          <div className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            <DialogTitle>Manage Permissions for {role.name}</DialogTitle>
          </div>
          <DialogDescription>
            Assign or remove permissions for this role. Permissions are grouped by resource.
          </DialogDescription>
        </DialogHeader>

        <div className="flex-1 min-h-0 overflow-y-auto space-y-4 pr-2">
          {/* Summary stats */}
          <div className="flex items-center gap-4 text-sm">
            <div className="flex items-center gap-2">
              <Badge variant="outline">
                {localAssignedIds.length} / {allPermissions.length}
              </Badge>
              <span className="text-muted-foreground">permissions assigned</span>
            </div>
            {saveMutation.isPending && (
              <div className="flex items-center gap-2 text-muted-foreground">
                <Loader2 className="h-4 w-4 animate-spin" />
                <span>Saving changes...</span>
              </div>
            )}
          </div>

          {/* Permission list */}
          <PermissionCheckboxList
            permissions={allPermissions}
            assignedPermissionIds={localAssignedIds}
            onTogglePermission={handleTogglePermission}
            isBuiltinRole={role.isBuiltin}
            disabled={saveMutation.isPending}
            className="space-y-6"
            data-testid="role-permissions-checkbox-list"
          />
        </div>

        {/* Footer actions */}
        <div className="flex-shrink-0 flex justify-end gap-2 pt-4 border-t">
          <Button
            variant="outline"
            onClick={() => {
              setLocalAssignedIds(assignedPermissionIds)
              onOpenChange(false)
            }}
            disabled={saveMutation.isPending}
            data-testid="role-permissions-cancel-button"
          >
            Cancel
          </Button>
          <Button
            onClick={() => saveMutation.mutate()}
            disabled={!hasChanges || saveMutation.isPending}
            data-testid="role-permissions-save-button"
          >
            {saveMutation.isPending ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
                Saving...
              </>
            ) : (
              'Save Changes'
            )}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
