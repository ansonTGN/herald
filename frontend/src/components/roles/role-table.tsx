import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import { BuiltinBadge } from '@/components/shared/builtin-badge'
import { Edit, Trash2, Shield } from 'lucide-react'
import type { RoleResponse, PermissionResponse } from '@/lib/api-generated'
import { EditRoleDialog } from './edit-role-dialog'
import { DeleteRoleDialog } from './delete-role-dialog'
import { RolePermissionsDialog } from './role-permissions-dialog'
import { useState } from 'react'
import { useRealmId } from '@/stores/auth-store'
import { useQueries } from '@tanstack/react-query'
import { permissionsQueryOptions, rolePermissionsQueryOptions } from '@/data/query-options'

interface RoleTableProps {
  roles: RoleResponse[]
  isLoading: boolean
  error: unknown
}

export function RoleTable({ roles, isLoading, error }: RoleTableProps) {
  const realmId = useRealmId()
  const [editingRole, setEditingRole] = useState<RoleResponse | null>(null)
  const [deletingRole, setDeletingRole] = useState<RoleResponse | null>(null)
  const [managingPermissionsRole, setManagingPermissionsRole] = useState<RoleResponse | null>(null)
  const [editDialogOpen, setEditDialogOpen] = useState(false)
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false)
  const [permissionsDialogOpen, setPermissionsDialogOpen] = useState(false)

  const handleEdit = (role: RoleResponse) => {
    setEditingRole(role)
    setEditDialogOpen(true)
  }

  const handleDelete = (role: RoleResponse) => {
    if (role.isBuiltin) {
      return // Should not happen due to UI hiding, but defensive check
    }
    setDeletingRole(role)
    setDeleteDialogOpen(true)
  }

  const handleManagePermissions = async (role: RoleResponse) => {
    setManagingPermissionsRole(role)
    setPermissionsDialogOpen(true)
  }

  if (isLoading) {
    return (
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Description</TableHead>
              <TableHead>Permissions</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell colSpan={4} className="text-center py-8 text-muted-foreground">
                Loading roles...
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>
    )
  }

  if (error) {
    return (
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Description</TableHead>
              <TableHead>Permissions</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell colSpan={4} className="text-center py-8 text-destructive">
                Failed to load roles. Please try again later.
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>
    )
  }

  if (roles.length === 0) {
    return (
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Description</TableHead>
              <TableHead>Permissions</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell colSpan={4} className="text-center py-8 text-muted-foreground">
                No roles found. Create your first role to get started.
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>
    )
  }

  return (
    <>
      <div className="rounded-md border" data-testid="role-table">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Description</TableHead>
              <TableHead>Permissions</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {roles.map((role) => (
              <TableRow key={role.id}>
                <TableCell className="font-medium">
                  <div className="flex items-center gap-2">
                    {role.name}
                    <BuiltinBadge isBuiltin={role.isBuiltin} />
                  </div>
                </TableCell>
                <TableCell className="text-muted-foreground max-w-md truncate">
                  {role.description || '-'}
                </TableCell>
                <TableCell>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="gap-1"
                    onClick={() => handleManagePermissions(role)}
                    data-testid={`role-permissions-button-${role.id}`}
                  >
                    <Shield className="h-3 w-3" />
                    Manage Permissions
                  </Button>
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex justify-end gap-2">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleEdit(role)}
                      data-testid={`role-edit-button-${role.id}`}
                    >
                      <Edit className="h-4 w-4" />
                    </Button>
                    {!role.isBuiltin && (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleDelete(role)}
                        data-testid={`role-delete-button-${role.id}`}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    )}
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {editingRole && (
        <EditRoleDialog
          open={editDialogOpen}
          onOpenChange={setEditDialogOpen}
          role={editingRole}
          realmId={editingRole.realmId}
        />
      )}

      {deletingRole && (
        <DeleteRoleDialog
          open={deleteDialogOpen}
          onOpenChange={setDeleteDialogOpen}
          role={deletingRole}
          realmId={deletingRole.realmId}
        />
      )}

      {managingPermissionsRole && (
        <RolePermissionsDataProvider
          realmId={realmId}
          roleId={managingPermissionsRole.id}
          open={permissionsDialogOpen}
          onOpenChange={setPermissionsDialogOpen}
          role={managingPermissionsRole}
        />
      )}
    </>
  )
}

// Inner component to handle data fetching for permissions dialog
function RolePermissionsDataProvider({
  realmId,
  roleId,
  open,
  onOpenChange,
  role,
}: {
  realmId: string
  roleId: string
  open: boolean
  onOpenChange: (open: boolean) => void
  role: RoleResponse
}) {
  const permissionsData = useQueries({
    queries: [permissionsQueryOptions(realmId), rolePermissionsQueryOptions(realmId, roleId)],
    combine: (queries) => {
      const [perms, rolePerms] = queries
      return {
        allPermissions: perms.data ?? [],
        assignedPermissionIds: (rolePerms.data ?? []).map((p: PermissionResponse) => p.id),
      }
    },
  })

  if (!open) return null

  return (
    <RolePermissionsDialog
      open={open}
      onOpenChange={onOpenChange}
      role={role}
      realmId={realmId}
      allPermissions={permissionsData.allPermissions}
      assignedPermissionIds={permissionsData.assignedPermissionIds}
    />
  )
}
