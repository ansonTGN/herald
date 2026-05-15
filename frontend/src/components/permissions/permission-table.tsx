import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { BuiltinBadge } from '@/components/shared/builtin-badge'
import { Edit, Trash2 } from 'lucide-react'
import type { PermissionResponse } from '@/lib/api-generated'
import { EditPermissionDialog } from './edit-permission-dialog'
import { DeletePermissionDialog } from './delete-permission-dialog'
import { useDialogManager } from '@/hooks/use-dialog-state'

interface PermissionTableProps {
  permissions: PermissionResponse[]
  isLoading: boolean
  error: unknown
}

export function PermissionTable({ permissions, isLoading, error }: PermissionTableProps) {
  const editDialog = useDialogManager<PermissionResponse>()
  const deleteDialog = useDialogManager<PermissionResponse>()

  const handleEdit = (permission: PermissionResponse) => {
    editDialog.open(permission)
  }

  const handleDelete = (permission: PermissionResponse) => {
    if (permission.isBuiltin) {
      return
    }
    deleteDialog.open(permission)
  }

  if (isLoading) {
    return (
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Resource</TableHead>
              <TableHead>Action</TableHead>
              <TableHead>Description</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                Loading permissions...
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
              <TableHead>Resource</TableHead>
              <TableHead>Action</TableHead>
              <TableHead>Description</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell colSpan={5} className="text-center py-8 text-destructive">
                Failed to load permissions. Please try again later.
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>
    )
  }

  if (permissions.length === 0) {
    return (
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Resource</TableHead>
              <TableHead>Action</TableHead>
              <TableHead>Description</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            <TableRow>
              <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                No permissions found. Create your first permission to get started.
              </TableCell>
            </TableRow>
          </TableBody>
        </Table>
      </div>
    )
  }

  return (
    <>
      <div className="rounded-md border" data-testid="permissions-table">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Resource</TableHead>
              <TableHead>Action</TableHead>
              <TableHead>Description</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {permissions.map((permission) => (
              <TableRow key={permission.id}>
                <TableCell className="font-medium">
                  <div className="flex items-center gap-2">
                    {permission.name}
                    <BuiltinBadge isBuiltin={permission.isBuiltin} />
                  </div>
                </TableCell>
                <TableCell>
                  <Badge variant="outline">{permission.resource}</Badge>
                </TableCell>
                <TableCell>
                  <Badge variant="secondary">{permission.action}</Badge>
                </TableCell>
                <TableCell className="text-muted-foreground max-w-md truncate">
                  {permission.description || '-'}
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex justify-end gap-2">
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => handleEdit(permission)}
                      disabled={permission.isBuiltin}
                      data-testid={`permission-edit-button-${permission.id}`}
                    >
                      <Edit className="h-4 w-4" />
                    </Button>
                    {!permission.isBuiltin && (
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleDelete(permission)}
                        data-testid={`permission-delete-button-${permission.id}`}
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

      {editDialog.selectedItem && (
        <EditPermissionDialog
          open={editDialog.isOpen}
          onOpenChange={editDialog.onOpenChange}
          permission={editDialog.selectedItem}
          realmId={editDialog.selectedItem.realmId}
        />
      )}

      {deleteDialog.selectedItem && (
        <DeletePermissionDialog
          open={deleteDialog.isOpen}
          onOpenChange={deleteDialog.onOpenChange}
          permission={deleteDialog.selectedItem}
          realmId={deleteDialog.selectedItem.realmId}
        />
      )}
    </>
  )
}
