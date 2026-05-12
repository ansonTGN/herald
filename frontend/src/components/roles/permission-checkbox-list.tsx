import { Checkbox } from '@/components/ui/checkbox'
import { Badge } from '@/components/ui/badge'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { BuiltinBadge } from '@/components/shared/builtin-badge'
import { Shield, AlertTriangle } from 'lucide-react'
import type { PermissionResponse } from '@/lib/api-generated'

interface PermissionCheckboxListProps {
  permissions: PermissionResponse[]
  assignedPermissionIds: string[]
  onTogglePermission: (permissionId: string, checked: boolean) => void
  isBuiltinRole: boolean
  disabled?: boolean
  dataTestId?: string
}

/**
 * Group permissions by resource for better organization
 */
function groupPermissionsByResource(
  permissions: PermissionResponse[]
): Map<string, PermissionResponse[]> {
  const grouped = new Map<string, PermissionResponse[]>()

  permissions.forEach((permission) => {
    const resource = permission.resource
    if (!grouped.has(resource)) {
      grouped.set(resource, [])
    }
    grouped.get(resource)!.push(permission)
  })

  return grouped
}

/**
 * Check if a permission is a built-in permission
 * Built-in permissions have specific IDs that are auto-generated
 */
function isBuiltinPermission(permission: PermissionResponse): boolean {
  return permission.isBuiltin
}

export function PermissionCheckboxList({
  permissions,
  assignedPermissionIds,
  onTogglePermission,
  isBuiltinRole,
  disabled = false,
  dataTestId = 'permission-checkbox-list',
}: PermissionCheckboxListProps) {
  const groupedPermissions = groupPermissionsByResource(permissions)

  const handleCheckedChange = (permissionId: string, checked: boolean | string) => {
    const isChecked = checked === true
    onTogglePermission(permissionId, isChecked)
  }

  if (permissions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
        <Shield className="h-12 w-12 mb-2 opacity-50" />
        <p>No permissions available</p>
      </div>
    )
  }

  return (
    <div className="max-h-[400px] overflow-y-auto pr-4" data-testid={dataTestId}>
      <div className="space-y-6">
        {isBuiltinRole && (
          <Alert variant="default">
            <AlertTriangle className="h-4 w-4" />
            <AlertDescription>
              Built-in permissions cannot be removed from built-in roles. You can still add custom
              permissions.
            </AlertDescription>
          </Alert>
        )}

        {Array.from(groupedPermissions.entries()).map(([resource, resourcePermissions]) => (
          <div key={resource} className="space-y-3">
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="font-semibold">
                {resource}
              </Badge>
              <span className="text-sm text-muted-foreground">
                ({resourcePermissions.length} permission
                {resourcePermissions.length !== 1 ? 's' : ''})
              </span>
            </div>

            <div className="space-y-2 pl-2">
              {resourcePermissions.map((permission) => {
                const isAssigned = assignedPermissionIds.includes(permission.id)
                const isBuiltin = isBuiltinPermission(permission)
                const isDisabled = disabled || (isBuiltinRole && isBuiltin && isAssigned)

                return (
                  <div
                    key={permission.id}
                    className="flex items-start gap-3 p-2 rounded-md hover:bg-accent/50 transition-colors"
                    data-testid={`permission-item-${permission.id}`}
                  >
                    <Checkbox
                      id={`permission-${permission.id}`}
                      checked={isAssigned}
                      onCheckedChange={(checked) => handleCheckedChange(permission.id, checked)}
                      disabled={isDisabled}
                      data-testid={`permission-checkbox-${permission.id}`}
                    />
                    <div className="flex-1 space-y-1">
                      <label
                        htmlFor={`permission-${permission.id}`}
                        className="text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 cursor-pointer"
                      >
                        <div className="flex items-center gap-2">
                          {permission.name}
                          <BuiltinBadge isBuiltin={isBuiltin} />
                        </div>
                      </label>
                      {permission.description && (
                        <p className="text-xs text-muted-foreground">{permission.description}</p>
                      )}
                    </div>
                  </div>
                )
              })}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
