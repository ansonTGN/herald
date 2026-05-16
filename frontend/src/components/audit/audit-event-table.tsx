import type { ColumnDef } from '@tanstack/react-table'
import { DataTable } from '@/components/shared'
import { Badge } from '@/components/ui/badge'
import type { AuditEventResponse } from '@/lib/api-generated'
import { formatDateTime } from '@/lib/date-utils'

interface AuditEventTableProps {
  data: AuditEventResponse[]
  onRowClick: (event: AuditEventResponse) => void
  emptyMessage?: string
}

const columns: ColumnDef<AuditEventResponse>[] = [
  {
    accessorKey: 'createdAt',
    header: 'Time',
    cell: ({ getValue }) => (
      <span className="whitespace-nowrap text-sm">{formatDateTime(getValue() as string)}</span>
    ),
  },
  {
    accessorKey: 'actorName',
    header: 'Actor',
    cell: ({ row }) => {
      const name = row.original.actorName
      const id = row.original.actorId
      return (
        <div className="max-w-[200px]">
          <div className="truncate text-sm font-medium">{name || 'Unknown'}</div>
          <div className="truncate text-xs text-muted-foreground">{id}</div>
        </div>
      )
    },
  },
  {
    accessorKey: 'category',
    header: 'Category',
    cell: ({ getValue }) => (
      <span className="text-sm">{(getValue() as string).replace(/_/g, ' ')}</span>
    ),
  },
  {
    accessorKey: 'action',
    header: 'Action',
    cell: ({ getValue }) => <span className="font-mono text-sm">{getValue() as string}</span>,
  },
  {
    id: 'target',
    header: 'Target',
    cell: ({ row }) => {
      const name = row.original.targetName
      const id = row.original.targetId
      const type = row.original.targetType
      return (
        <div className="max-w-[200px]">
          <div className="truncate text-sm font-medium">{name || 'Unknown'}</div>
          <div className="truncate text-xs text-muted-foreground">
            {type}: {id}
          </div>
        </div>
      )
    },
  },
  {
    accessorKey: 'result',
    header: 'Result',
    cell: ({ getValue }) => {
      const result = getValue() as string
      const isSuccess = result === 'success'
      return (
        <Badge
          variant={isSuccess ? 'default' : 'destructive'}
          className="text-xs"
          data-testid={`audit-result-${result}`}
        >
          {result}
        </Badge>
      )
    },
  },
  {
    accessorKey: 'ipAddress',
    header: 'IP Address',
    cell: ({ getValue }) => (
      <span className="font-mono text-sm">{(getValue() as string) || '-'}</span>
    ),
  },
]

export function AuditEventTable({
  data,
  onRowClick,
  emptyMessage = 'No audit logs yet.',
}: AuditEventTableProps) {
  return (
    <DataTable
      columns={columns}
      data={data}
      onRowClick={onRowClick}
      emptyMessage={emptyMessage}
      data-testid="audit-table"
    />
  )
}
