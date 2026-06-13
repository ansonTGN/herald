import type { ColumnDef } from '@tanstack/react-table'
import { DataTable } from '@/components/shared'
import { Badge } from '@/components/ui/badge'
import type { AuditEventResponse } from '@/lib/api-generated'
import { formatDateTimeShort } from '@/lib/date-utils'
import { m } from '@/paraglide/messages'

interface AuditEventTableProps {
  data: AuditEventResponse[]
  onRowClick: (event: AuditEventResponse) => void
  emptyMessage?: string
}

const columns: ColumnDef<AuditEventResponse>[] = [
  {
    accessorKey: 'createdAt',
    header: () => m['audit.table_time'](),
    cell: ({ getValue }) => (
      <span className="whitespace-nowrap text-sm font-mono">
        {formatDateTimeShort(getValue() as string)}
      </span>
    ),
  },
  {
    accessorKey: 'actorName',
    header: () => m['audit.table_actor'](),
    cell: ({ row }) => {
      const name = row.original.actorName
      const id = row.original.actorId
      return (
        <div className="max-w-[200px]">
          <div className="truncate text-sm font-medium">{name || m['audit.unknown']()}</div>
          <div className="truncate text-xs text-muted-foreground">{id}</div>
        </div>
      )
    },
  },
  {
    accessorKey: 'category',
    header: () => m['audit.table_category'](),
    cell: ({ getValue }) => (
      <span className="text-sm">{(getValue() as string).replace(/_/g, ' ')}</span>
    ),
  },
  {
    accessorKey: 'action',
    header: () => m['audit.table_action'](),
    cell: ({ getValue }) => <span className="font-mono text-sm">{getValue() as string}</span>,
  },
  {
    id: 'target',
    header: () => m['audit.table_target'](),
    cell: ({ row }) => {
      const name = row.original.targetName
      const id = row.original.targetId
      const type = row.original.targetType
      return (
        <div className="max-w-[200px]">
          <div className="truncate text-sm font-medium">{name || m['audit.unknown']()}</div>
          <div className="truncate text-xs text-muted-foreground">
            {type}: {id}
          </div>
        </div>
      )
    },
  },
  {
    accessorKey: 'result',
    header: () => m['audit.table_result'](),
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
    header: () => m['audit.table_ip_address'](),
    cell: ({ getValue }) => (
      <span className="font-mono text-sm">{(getValue() as string) || '-'}</span>
    ),
  },
]

export function AuditEventTable({
  data,
  onRowClick,
  emptyMessage = m['audit.no_logs'](),
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
