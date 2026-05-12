import { memo, useMemo } from 'react'
import { format } from 'date-fns'
import { ChevronRight, Calendar, Coins, CreditCard, AlertCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Skeleton } from '@/components/ui/skeleton'
import type { PurchaseHistoryItemDto } from '@/lib/api-generated'

interface PurchaseHistoryListProps {
  purchases: PurchaseHistoryItemDto[]
  isLoading: boolean
  error?: Error
  pagination?: {
    page: number
    pageSize: number
    total: number
    totalPages: number
  }
  onPageChange?: (page: number) => void
  onDetailsClick: (purchaseId: string) => void
}

interface HistoryTableRowProps {
  purchase: PurchaseHistoryItemDto
  onDetailsClick: (purchaseId: string) => void
}

const HistoryTableRow = memo(({ purchase, onDetailsClick }: HistoryTableRowProps) => {
  const timestamp = useMemo(() => {
    try {
      return format(new Date(purchase.createdAt), 'PPp')
    } catch {
      return purchase.createdAt
    }
  }, [purchase.createdAt])

  return (
    <TableRow data-testid={`purchase-history-item-${purchase.id}`}>
      <TableCell className="font-medium">{timestamp}</TableCell>
      <TableCell>
        {purchase.pointsPackageId ? (
          <div>
            <div className="text-sm font-medium">Package #{purchase.pointsPackageId}</div>
            <div className="text-xs text-muted-foreground">
              {purchase.points.toLocaleString()} points
            </div>
          </div>
        ) : (
          <span className="text-sm text-muted-foreground">-</span>
        )}
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-2">
          <Coins className="h-4 w-4 text-primary" />
          <span className="font-medium">{purchase.points.toLocaleString()}</span>
        </div>
      </TableCell>
      <TableCell>
        <div className="text-sm">
          {purchase.amount.toFixed(2)} {purchase.currency}
        </div>
      </TableCell>
      <TableCell>
        <div className="flex items-center gap-2">
          <CreditCard className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm">{purchase.paymentProvider}</span>
        </div>
      </TableCell>
      <TableCell className="text-right">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onDetailsClick(purchase.id)}
          data-testid={`purchase-history-details-button-${purchase.id}`}
        >
          <ChevronRight className="h-4 w-4" />
          <span className="sr-only">View details</span>
        </Button>
      </TableCell>
    </TableRow>
  )
})

HistoryTableRow.displayName = 'HistoryTableRow'

export function PurchaseHistoryList({
  purchases,
  isLoading,
  error,
  pagination,
  onPageChange,
  onDetailsClick,
}: PurchaseHistoryListProps) {
  if (isLoading) {
    return (
      <div data-testid="purchase-history-loading" className="space-y-4">
        {[...Array(5)].map((_, i) => (
          <div key={i} className="flex items-center space-x-4">
            <Skeleton className="h-12 w-full" />
          </div>
        ))}
      </div>
    )
  }

  if (error) {
    return (
      <div
        data-testid="purchase-history-error"
        className="flex flex-col items-center justify-center py-12 text-center"
      >
        <AlertCircle className="mb-4 h-12 w-12 text-destructive" />
        <h3 className="text-lg font-semibold">Failed to load purchase history</h3>
        <p className="text-sm text-muted-foreground">{error.message}</p>
      </div>
    )
  }

  if (!purchases || purchases.length === 0) {
    return (
      <div
        data-testid="purchase-history-empty"
        className="flex flex-col items-center justify-center py-12 text-center"
      >
        <Calendar className="mb-4 h-12 w-12 text-muted-foreground" />
        <h3 className="text-lg font-semibold">No purchase history</h3>
        <p className="text-sm text-muted-foreground">
          You haven't purchased any points packages yet
        </p>
      </div>
    )
  }

  const totalPages = pagination?.totalPages || 1
  const currentPage = pagination?.page || 1

  return (
    <div className="space-y-4">
      <div data-testid="purchase-history-list" className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Date</TableHead>
              <TableHead>Package</TableHead>
              <TableHead>Points</TableHead>
              <TableHead>Amount</TableHead>
              <TableHead>Provider</TableHead>
              <TableHead className="text-right">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {purchases.map((purchase) => (
              <HistoryTableRow
                key={purchase.id}
                purchase={purchase}
                onDetailsClick={onDetailsClick}
              />
            ))}
          </TableBody>
        </Table>
      </div>

      {pagination && totalPages > 1 && (
        <div className="flex items-center justify-between">
          <p className="text-sm text-muted-foreground">
            Page {currentPage} of {totalPages} ({pagination.total} total)
          </p>
          <div className="flex items-center gap-1">
            <Button
              variant="outline"
              size="sm"
              onClick={() => onPageChange?.(currentPage - 1)}
              disabled={currentPage === 1}
              className="gap-1"
            >
              Previous
            </Button>
            {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => (
              <Button
                key={page}
                variant={page === currentPage ? 'default' : 'outline'}
                size="sm"
                onClick={() => onPageChange?.(page)}
              >
                {page}
              </Button>
            ))}
            <Button
              variant="outline"
              size="sm"
              onClick={() => onPageChange?.(currentPage + 1)}
              disabled={currentPage === totalPages}
              className="gap-1"
            >
              Next
            </Button>
          </div>
        </div>
      )}
    </div>
  )
}
