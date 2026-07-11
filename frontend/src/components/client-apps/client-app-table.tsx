import { type ColumnDef, flexRender, getCoreRowModel, useReactTable } from '@tanstack/react-table'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import type { ClientAppItem } from '@/lib/api-generated'
import { Badge } from '@/components/ui/badge'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { m } from '@/paraglide/messages'
import { getErrorMessage } from '@/lib/error-utils'

interface ClientAppTableProps {
  data?: ClientAppItem[]
  isLoading?: boolean
  error?: unknown
  onEdit?: (clientApp: ClientAppItem) => void
  onDelete?: (clientApp: ClientAppItem) => void
  onToggleEnabled?: (clientApp: ClientAppItem) => void
  canUpdate?: boolean
  canDelete?: boolean
}

function createClientAppColumns(
  onEdit?: (clientApp: ClientAppItem) => void,
  onDelete?: (clientApp: ClientAppItem) => void,
  onToggleEnabled?: (clientApp: ClientAppItem) => void,
  canUpdate = true,
  canDelete = true
): ColumnDef<ClientAppItem>[] {
  return [
    {
      id: 'icon',
      header: m['client_apps.table_icon'](),
      cell: ({ row }) =>
        row.original.iconUrl ? (
          <img
            src={row.original.iconUrl}
            alt={row.original.name}
            className="w-8 h-8 rounded"
            data-testid="client-app-icon"
          />
        ) : (
          <div className="w-8 h-8 bg-gray-200 rounded flex items-center justify-center text-gray-500 text-xs">
            N/A
          </div>
        ),
    },
    {
      id: 'clientId',
      accessorKey: 'clientId',
      header: m['client_apps.table_client_id'](),
      cell: ({ row }) => (
        <span className="font-mono text-sm" data-testid="client-app-client-id">
          {row.getValue('clientId')}
        </span>
      ),
    },
    {
      id: 'name',
      accessorKey: 'name',
      header: m['client_apps.table_name'](),
      cell: ({ row }) => row.getValue('name'),
    },
    {
      id: 'redirectUris',
      header: m['client_apps.table_redirect_uris'](),
      cell: ({ row }) => (
        <div
          className="max-w-xs truncate"
          data-testid="client-app-redirect-uris"
          title={row.original.redirectUris.join(', ')}
        >
          {row.original.redirectUris.join(', ')}
        </div>
      ),
    },
    {
      id: 'sessionTtlSeconds',
      header: m['client_apps.table_session_ttl'](),
      cell: ({ row }) => (
        <span data-testid="client-app-session-ttl">
          {m['client_apps.session_ttl_minutes']({
            minutes: Math.floor(row.original.sessionTtlSeconds / 60),
          })}
        </span>
      ),
    },
    {
      id: 'enabled',
      header: m['client_apps.table_status'](),
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <Switch
            checked={row.original.enabled}
            onCheckedChange={() => onToggleEnabled?.(row.original)}
            data-testid="client-app-enabled-switch"
          />
          <Badge
            variant={row.original.enabled ? 'default' : 'secondary'}
            data-testid="client-app-status-badge"
          >
            {row.original.enabled
              ? m['client_apps.status_enabled_label']()
              : m['client_apps.status_disabled_label']()}
          </Badge>
        </div>
      ),
    },
    {
      id: 'actions',
      header: m['client_apps.table_actions'](),
      cell: ({ row }) => (
        <div className="flex gap-2" data-testid="client-app-actions">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onEdit?.(row.original)}
            data-testid="edit-client-app-button"
            disabled={!canUpdate}
            title={!canUpdate ? m['client_apps.edit_disabled_title']() : undefined}
          >
            {m['client_apps.edit_button']()}
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onDelete?.(row.original)}
            data-testid="delete-client-app-button"
            disabled={!canDelete}
            title={!canDelete ? m['client_apps.delete_disabled_title']() : undefined}
          >
            {m['client_apps.delete_button']()}
          </Button>
        </div>
      ),
    },
  ]
}

export function ClientAppTable({
  data,
  isLoading = false,
  error,
  onEdit,
  onDelete,
  onToggleEnabled,
  canUpdate = true,
  canDelete = true,
}: ClientAppTableProps) {
  const columns = createClientAppColumns(onEdit, onDelete, onToggleEnabled, canUpdate, canDelete)

  const table = useReactTable({
    data: data ?? [],
    columns,
    getCoreRowModel: getCoreRowModel(),
  })

  if (isLoading) {
    return (
      <div className="rounded-md border p-8">
        <div className="flex items-center justify-center">
          <div className="text-muted-foreground">{m['client_apps.loading']()}</div>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="rounded-md border p-8">
        <div className="flex items-center justify-center text-red-500">
          {m['client_apps.error_loading']({ message: getErrorMessage(error) })}
        </div>
      </div>
    )
  }

  if (!data || data.length === 0) {
    return (
      <div className="rounded-md border p-8">
        <div className="flex items-center justify-center text-muted-foreground">
          {m['client_apps.empty']()}
        </div>
      </div>
    )
  }

  return (
    <div className="rounded-md border">
      <Table data-testid="client-apps-table">
        <TableHeader>
          {table.getHeaderGroups().map((headerGroup) => (
            <TableRow key={headerGroup.id}>
              {headerGroup.headers.map((header) => (
                <TableHead key={header.id}>
                  {header.isPlaceholder
                    ? null
                    : flexRender(header.column.columnDef.header, header.getContext())}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows?.length ? (
            table.getRowModel().rows.map((row) => (
              <TableRow
                key={row.id}
                data-state={row.getIsSelected() && 'selected'}
                data-testid={`client-app-row-${row.index}`}
                data-client-id={row.getValue('clientId')}
                data-app-id={row.original.id}
              >
                {row.getVisibleCells().map((cell) => (
                  <TableCell key={cell.id}>
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </TableCell>
                ))}
              </TableRow>
            ))
          ) : (
            <TableRow>
              <TableCell colSpan={columns.length} className="h-24 text-center">
                {m['client_apps.no_results']()}
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  )
}
