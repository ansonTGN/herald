import { type ColumnDef, flexRender, getCoreRowModel, useReactTable } from '@tanstack/react-table'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { m } from '@/paraglide/messages'

export interface DataTableProps<TData, TValue> {
  columns: ColumnDef<TData, TValue>[]
  data: TData[]
  isLoading?: boolean
  error?: Error
  loadingMessage?: string
  errorMessage?: string
  emptyMessage?: string
  onRowClick?: (row: TData) => void
  'data-testid'?: string
}

const coreRowModel = getCoreRowModel()

export function DataTable<TData, TValue>({
  columns,
  data,
  isLoading = false,
  error,
  loadingMessage,
  errorMessage,
  emptyMessage,
  onRowClick,
  'data-testid': dataTestId,
}: DataTableProps<TData, TValue>) {
  const table = useReactTable({
    data,
    columns,
    getCoreRowModel: coreRowModel,
  })

  const testIdProps = dataTestId ? { 'data-testid': dataTestId } : undefined

  const resolvedLoadingMessage = loadingMessage ?? m['common.loading']()
  const resolvedEmptyMessage = emptyMessage ?? m['common.no_results']()

  if (isLoading) {
    return (
      <div className="py-8 text-center" {...testIdProps}>
        {resolvedLoadingMessage}
      </div>
    )
  }

  if (error) {
    return (
      <div className="py-8 text-center text-destructive" {...testIdProps}>
        {errorMessage ?? m['error.error_prefix']({ message: error.message })}
      </div>
    )
  }

  if (data.length === 0) {
    return (
      <div className="py-8 text-center text-muted-foreground" {...testIdProps}>
        {resolvedEmptyMessage}
      </div>
    )
  }

  return (
    <div className="rounded-md border" {...testIdProps}>
      <Table>
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
          {table.getRowModel().rows.map((row) => (
            <TableRow
              key={row.id}
              data-state={row.getIsSelected() && 'selected'}
              className={onRowClick ? 'cursor-pointer' : undefined}
              onClick={() => onRowClick?.(row.original)}
            >
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}
