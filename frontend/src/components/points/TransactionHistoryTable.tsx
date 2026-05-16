import { useMemo, useCallback } from 'react'
import { type ColumnDef, flexRender, getCoreRowModel, useReactTable } from '@tanstack/react-table'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { ArrowUpRight, ArrowDownRight, Clock, ExternalLink } from 'lucide-react'
import type { PointsTransactionResponse } from '@/lib/api-generated'
import type { TransactionFilters } from '@/lib/schemas/points-forms'

interface TransactionHistoryTableProps {
  transactions: PointsTransactionResponse[]
  loading?: boolean
  filters: TransactionFilters
  admin?: boolean
  clientApps?: Array<{ id: string; name: string }>
}

export function TransactionHistoryTable({
  transactions,
  loading = false,
  filters,
  admin = false,
  clientApps,
}: TransactionHistoryTableProps) {
  // Memoized date formatter to avoid re-creating date objects on every render
  const formatDate = useCallback((dateStr: string) => {
    const date = new Date(dateStr)
    return `${date.toLocaleDateString()} ${date.toLocaleTimeString()}`
  }, [])

  // Memoized filter check to avoid computing on every render
  const hasActiveFilters = useMemo(
    () =>
      !!filters.transactionType ||
      !!filters.startTime ||
      !!filters.endTime ||
      !!filters.clientAppId,
    [filters]
  )

  // Memoized client app map for O(1) lookup
  const clientAppsMap = useMemo(
    () => new Map(clientApps?.map((app) => [app.id, app])),
    [clientApps]
  )

  const columns = useMemo<ColumnDef<PointsTransactionResponse>[]>(
    () => [
      {
        id: 'createdAt',
        accessorKey: 'createdAt',
        header: 'Time',
        cell: ({ row }) => {
          const dateStr = row.getValue('createdAt') as string
          return (
            <div className="flex items-center gap-2" data-testid={`transaction-time-${row.index}`}>
              <Clock className="h-3 w-3 text-muted-foreground" />
              <span className="text-sm">{formatDate(dateStr)}</span>
            </div>
          )
        },
      },
      {
        accessorKey: 'transactionType',
        header: 'Type',
        cell: ({ row }) => {
          // Since API uses 'type' field, we need to determine from amount sign
          const amount = row.getValue('amount') as number
          const type = amount >= 0 ? 'recharge' : 'consume'
          return (
            <div className="flex items-center gap-2" data-testid={`transaction-type-${row.index}`}>
              {type === 'recharge' ? (
                <ArrowUpRight className="h-4 w-4 text-green-600" />
              ) : (
                <ArrowDownRight className="h-4 w-4 text-red-600" />
              )}
              <Badge variant={type === 'recharge' ? 'default' : 'secondary'}>
                {type === 'recharge' ? 'Recharge' : 'Consume'}
              </Badge>
            </div>
          )
        },
      },
      {
        accessorKey: 'amount',
        header: 'Amount',
        cell: ({ row }) => {
          const amount = row.getValue('amount') as number
          return (
            <div
              className={`font-semibold ${amount >= 0 ? 'text-green-600' : 'text-red-600'}`}
              data-testid={`transaction-amount-${row.index}`}
            >
              {amount >= 0 ? '+' : ''}
              {amount.toLocaleString()}
            </div>
          )
        },
      },
      {
        accessorKey: 'balanceAfter',
        header: 'Balance After',
        cell: ({ row }) => (
          <div className="font-mono" data-testid={`transaction-balance-${row.index}`}>
            {(row.getValue('balanceAfter') as number).toLocaleString()}
          </div>
        ),
      },
      {
        accessorKey: 'description',
        header: 'Description',
        cell: ({ row }) => {
          const description = row.getValue('description') as string | null
          const subscriptionId = row.original.subscriptionId
          return (
            <div className="max-w-xs truncate" data-testid={`transaction-description-${row.index}`}>
              {description || '-'}
              {subscriptionId && (
                <div className="text-xs text-muted-foreground">
                  Sub: {String(subscriptionId).slice(0, 8)}...
                </div>
              )}
            </div>
          )
        },
      },
      ...(admin
        ? [
            {
              accessorKey: 'clientAppId',
              header: 'Source',
              cell: ({ row }: { row: { getValue: (key: string) => unknown; index: number } }) => {
                const clientAppId = row.getValue('clientAppId') as string | null
                const clientApp = clientAppsMap.get(clientAppId ?? '')
                return (
                  <div data-testid={`transaction-client-${row.index}`}>
                    {clientApp
                      ? clientApp.name
                      : clientAppId
                        ? String(clientAppId).slice(0, 8) + '...'
                        : '-'}
                  </div>
                )
              },
            },
          ]
        : []),
      {
        accessorKey: 'externalRefId',
        header: 'Ref ID',
        cell: ({ row }) => {
          const externalRefId = row.getValue('externalRefId') as string | null
          return (
            <div
              className="text-xs text-muted-foreground"
              data-testid={`transaction-ref-${row.index}`}
            >
              {externalRefId ? (
                <div className="flex items-center gap-1">
                  <ExternalLink className="h-3 w-3" />
                  <span className="font-mono">{String(externalRefId).slice(0, 12)}...</span>
                </div>
              ) : (
                '-'
              )}
            </div>
          )
        },
      },
    ],
    [admin, clientAppsMap, formatDate]
  )

  // eslint-disable-next-line react-hooks/incompatible-library
  const table = useReactTable({
    data: transactions,
    columns,
    getCoreRowModel: getCoreRowModel(),
  })

  if (loading) {
    return (
      <div className="space-y-4">
        <div className="animate-pulse">
          <div className="h-12 bg-muted rounded mb-4" />
          {[...Array(5)].map((_, i) => (
            <div key={i} className="h-12 bg-muted/50 rounded mb-2" />
          ))}
        </div>
      </div>
    )
  }

  if (transactions.length === 0) {
    return (
      <div className="text-center py-12 text-muted-foreground" data-testid="no-transactions">
        <Clock className="h-12 w-12 mx-auto mb-4 opacity-50" />
        <p>No transactions found matching your criteria</p>
        {hasActiveFilters && <p className="text-sm mt-2">Try adjusting your filters</p>}
      </div>
    )
  }

  return (
    <div className="space-y-4" data-testid="transaction-history-table">
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
            <TableRow key={row.id} data-testid={`transaction-row-${row.index}`}>
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
