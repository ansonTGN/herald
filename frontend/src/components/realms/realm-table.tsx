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

interface RealmTableProps {
  data?: RealmResponse[]
  isLoading?: boolean
  error?: Error
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
            Realm ID
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
            Name
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </button>
        )
      },
      cell: ({ row }) => row.getValue('name') || '-',
    },
    {
      accessorKey: 'description',
      header: 'Description',
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
            Created At
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
            Updated At
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </button>
        )
      },
      cell: ({ row }) => formatDate(row.getValue('updatedAt') as string),
    },
    {
      id: 'actions',
      header: 'Actions',
      cell: ({ row }) => (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onViewDetail?.(row.original)}
          data-testid={`realm-${row.index}-edit-button`}
        >
          Edit
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
    return <div data-testid="realm-table-loading">Loading...</div>
  }

  if (error) {
    return (
      <div
        data-testid="realm-table-error"
        className="text-red-500 p-4 border border-red-200 rounded-md bg-red-50"
      >
        <div className="font-semibold">Failed to load realms</div>
        <div className="text-sm mt-1">
          {error.message || 'An unknown error occurred. Please try again later.'}
        </div>
      </div>
    )
  }

  if (!data || data.length === 0) {
    return <div data-testid="realm-table-empty">No realms found</div>
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
                No results.
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  )
}
