import {
  type ColumnDef,
  flexRender,
  getCoreRowModel,
  useReactTable,
  getSortedRowModel,
} from '@tanstack/react-table'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import type { RealmResponse } from '@/lib/api-generated'
import { Button } from '@/components/ui/button'
import { ArrowUpDown } from 'lucide-react'
import { formatDate } from '@/lib/date-utils'
import { m } from '@/paraglide/messages'
import { getErrorMessage } from '@/lib/error-utils'

interface RealmTableProps {
  data?: RealmResponse[]
  isLoading?: boolean
  error?: unknown
  onViewDetail?: (realm: RealmResponse) => void
}

function createRealmColumns(
  onViewDetail?: (realm: RealmResponse) => void
): ColumnDef<RealmResponse>[] {
  return [
    {
      accessorKey: 'id',
      header: ({ column }) => {
        const isSorted = column.getIsSorted()
        return (
          <button
            onClick={() => column.toggleSorting(isSorted === 'asc')}
            className="flex items-center gap-2 hover:text-accent-foreground"
            data-testid="realm-id-sort-button"
          >
            {m['realms.table_id']()}
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </button>
        )
      },
      cell: ({ row }) => (
        <div className="font-mono text-xs" data-testid={`realm-${row.index}-id`}>
          {row.getValue('id')}
        </div>
      ),
    },
    {
      accessorKey: 'name',
      header: ({ column }) => {
        const isSorted = column.getIsSorted()
        return (
          <button
            onClick={() => column.toggleSorting(isSorted === 'asc')}
            className="flex items-center gap-2 hover:text-accent-foreground"
            data-testid="realm-name-sort-button"
          >
            {m['realms.table_name']()}
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </button>
        )
      },
      cell: ({ row }) => row.getValue('name') || '-',
    },
    {
      accessorKey: 'description',
      header: m['realms.table_description'](),
      cell: ({ row }) => {
        const desc = row.getValue('description') as string | null | undefined
        if (!desc) return <span className="text-muted-foreground">&mdash;</span>
        return (
          <span className="line-clamp-2 max-w-[200px]" title={desc}>
            {desc.length > 50 ? desc.slice(0, 50) + '...' : desc}
          </span>
        )
      },
    },
    {
      accessorKey: 'createdAt',
      header: ({ column }) => {
        const isSorted = column.getIsSorted()
        return (
          <button
            onClick={() => column.toggleSorting(isSorted === 'asc')}
            className="flex items-center gap-2 hover:text-accent-foreground"
            data-testid="realm-created-at-sort-button"
          >
            {m['realms.table_created_at']()}
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </button>
        )
      },
      cell: ({ row }) => formatDate(row.getValue('createdAt') as string),
    },
    {
      accessorKey: 'updatedAt',
      header: ({ column }) => {
        const isSorted = column.getIsSorted()
        return (
          <button
            onClick={() => column.toggleSorting(isSorted === 'asc')}
            className="flex items-center gap-2 hover:text-accent-foreground"
            data-testid="realm-updated-at-sort-button"
          >
            {m['realms.table_updated_at']()}
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </button>
        )
      },
      cell: ({ row }) => formatDate(row.getValue('updatedAt') as string),
    },
    {
      id: 'actions',
      header: m['realms.table_actions'](),
      cell: ({ row }) => (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onViewDetail?.(row.original)}
          data-testid={`realm-${row.index}-edit-button`}
        >
          {m['realms.table_edit']()}
        </Button>
      ),
    },
  ]
}

export function RealmTable({ data, isLoading, error, onViewDetail }: RealmTableProps) {
  const columns = createRealmColumns(onViewDetail)

  const table = useReactTable({
    data: data ?? [],
    columns,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  })

  if (isLoading) {
    return <div data-testid="realm-table-loading">{m['common.loading']()}</div>
  }

  if (error) {
    return (
      <div
        data-testid="realm-table-error"
        className="text-destructive p-4 border border-destructive/20 rounded-md bg-destructive/10"
      >
        <div className="font-semibold">{m['realms.failed_to_load']()}</div>
        <div className="text-sm mt-1">
          {getErrorMessage(error) || m['realms.failed_to_load_detail']()}
        </div>
      </div>
    )
  }

  if (!data || data.length === 0) {
    return <div data-testid="realm-table-empty">{m['realms.no_realms_found']()}</div>
  }

  return (
    <div className="rounded-md border">
      <Table data-testid="realms-table">
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
              <TableRow key={row.id} data-state={row.getIsSelected() && 'selected'}>
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
                {m['common.no_results']()}
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  )
}
