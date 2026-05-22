import { type ColumnDef } from '@tanstack/react-table'
import { DataTable } from '@/components/shared'
import type { ApiKeyListItem } from '@/lib/api-generated'
import { Badge } from '@/components/ui/badge'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { formatDate } from '@/lib/date-utils'

interface ApiKeyTableProps {
  data?: ApiKeyListItem[]
  isLoading?: boolean
  error?: Error | null
  onEdit?: (apiKey: ApiKeyListItem) => void
  onDelete?: (apiKey: ApiKeyListItem) => void
  onToggleEnabled?: (apiKey: ApiKeyListItem) => void
  canUpdate?: boolean
  canDelete?: boolean
}

function createApiKeyColumns(
  onEdit?: (apiKey: ApiKeyListItem) => void,
  onDelete?: (apiKey: ApiKeyListItem) => void,
  onToggleEnabled?: (apiKey: ApiKeyListItem) => void,
  canUpdate = true,
  canDelete = true
): ColumnDef<ApiKeyListItem>[] {
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
}: ApiKeyTableProps) {
  const columns = createApiKeyColumns(onEdit, onDelete, onToggleEnabled, canUpdate, canDelete)

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
