import { type ColumnDef } from '@tanstack/react-table'
import { DataTable } from '@/components/shared'
import type { ApiKeyListItem } from '@/lib/api-generated'
import { Badge } from '@/components/ui/badge'
import { Skeleton } from '@/components/ui/skeleton'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { formatDate } from '@/lib/date-utils'

interface ApiKeyColumnConfig {
  onEdit?: (apiKey: ApiKeyListItem) => void
  onDelete?: (apiKey: ApiKeyListItem) => void
  onToggleEnabled?: (apiKey: ApiKeyListItem) => void
  canUpdate?: boolean
  canDelete?: boolean
  onManageRoles?: (apiKey: ApiKeyListItem) => void
  canManageRoles?: boolean
  isLoading?: boolean
}

interface ApiKeyTableProps extends ApiKeyColumnConfig {
  data?: ApiKeyListItem[]
  isLoading?: boolean
  error?: Error | null
}

function createApiKeyColumns(config: ApiKeyColumnConfig): ColumnDef<ApiKeyListItem>[] {
  const {
    onEdit,
    onDelete,
    onToggleEnabled,
    canUpdate = true,
    canDelete = true,
    onManageRoles,
    canManageRoles,
    isLoading,
  } = config
  return [
    {
      id: 'name',
      accessorKey: 'name',
      header: 'Name',
      cell: ({ row }) => (
        <span className="font-medium" data-testid="api-key-name">
          {row.getValue('name')}
        </span>
      ),
    },
    {
      id: 'roles',
      header: 'Roles',
      cell: ({ row }) => {
        if (isLoading) {
          return <Skeleton className="h-5 w-20" />
        }

        const roles = row.original.roles
        if (!roles?.length) {
          return <span data-testid="api-key-roles-cell">&mdash;</span>
        }

        const visible = roles.slice(0, 2)
        const remaining = roles.length - visible.length

        return (
          <div className="flex flex-wrap gap-1" data-testid="api-key-roles-cell">
            {visible.map((role) => (
              <Badge key={role.id} variant="secondary">
                {role.name}
              </Badge>
            ))}
            {remaining > 0 && (
              <Badge variant="outline" data-testid="api-key-roles-overflow">
                +{remaining} more
              </Badge>
            )}
          </div>
        )
      },
    },
    {
      id: 'enabled',
      header: 'Status',
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <Switch
            checked={row.original.enabled}
            onCheckedChange={() => onToggleEnabled?.(row.original)}
            data-testid="api-key-enabled-switch"
          />
          <Badge
            variant={row.original.enabled ? 'default' : 'secondary'}
            data-testid="api-key-status-badge"
          >
            {row.original.enabled ? 'Enabled' : 'Disabled'}
          </Badge>
        </div>
      ),
    },
    {
      id: 'expiresAt',
      header: 'Expires',
      cell: ({ row }) => (
        <span data-testid="api-key-expires">
          {row.original.expiresAt ? formatDate(row.original.expiresAt) : 'Never'}
        </span>
      ),
    },
    {
      id: 'lastUsedAt',
      header: 'Last Used',
      cell: ({ row }) => (
        <span data-testid="api-key-last-used">
          {row.original.lastUsedAt ? formatDate(row.original.lastUsedAt) : 'Never'}
        </span>
      ),
    },
    {
      id: 'usageCount',
      accessorKey: 'usageCount',
      header: 'Usage Count',
      cell: ({ row }) => (
        <span data-testid="api-key-usage-count">{row.getValue('usageCount')}</span>
      ),
    },
    {
      id: 'actions',
      header: 'Actions',
      cell: ({ row }) => (
        <div className="flex gap-2" data-testid="api-key-actions">
          {canManageRoles && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onManageRoles?.(row.original)}
              data-testid="manage-api-key-roles-button"
            >
              Roles
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onEdit?.(row.original)}
            data-testid="edit-api-key-button"
            disabled={!canUpdate}
            title={!canUpdate ? 'You do not have permission to edit API keys' : undefined}
          >
            Edit
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onDelete?.(row.original)}
            data-testid="delete-api-key-button"
            disabled={!canDelete}
            title={!canDelete ? 'You do not have permission to delete API keys' : undefined}
          >
            Delete
          </Button>
        </div>
      ),
    },
  ]
}

export function ApiKeyTable({
  data,
  isLoading = false,
  error,
  onEdit,
  onDelete,
  onToggleEnabled,
  canUpdate = true,
  canDelete = true,
  onManageRoles,
  canManageRoles,
}: ApiKeyTableProps) {
  const columns = createApiKeyColumns({
    onEdit,
    onDelete,
    onToggleEnabled,
    canUpdate,
    canDelete,
    onManageRoles,
    canManageRoles,
    isLoading,
  })

  return (
    <DataTable
      columns={columns}
      data={data ?? []}
      isLoading={isLoading}
      error={error ?? undefined}
      loadingMessage="Loading API keys..."
      emptyMessage="No API keys found. Create your first API key to get started."
      data-testid="api-keys-table"
    />
  )
}
