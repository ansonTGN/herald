import { m } from '@/paraglide/messages'
import { memo, useMemo } from 'react'
import { format } from 'date-fns'
import { ChevronRight, Calendar, Coins, CreditCard, AlertCircle, FileText } from 'lucide-react'
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
  onDetailsClick: (purchaseId: string) => void
  onApplyInvoice?: (paymentAttemptId: string) => void
}

interface HistoryTableRowProps {
  purchase: PurchaseHistoryItemDto
  onDetailsClick: (purchaseId: string) => void
  onApplyInvoice?: (paymentAttemptId: string) => void
}

const HistoryTableRow = memo(
  ({ purchase, onDetailsClick, onApplyInvoice }: HistoryTableRowProps) => {
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
          <div className="flex justify-end gap-1">
            {onApplyInvoice && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => onApplyInvoice(purchase.paymentAttemptId)}
                data-testid={`purchase-history-invoice-button-${purchase.id}`}
              >
                <FileText className="h-4 w-4" />
                <span className="sr-only">Apply for invoice</span>
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => onDetailsClick(purchase.id)}
              data-testid={`purchase-history-details-button-${purchase.id}`}
            >
              <ChevronRight className="h-4 w-4" />
              <span className="sr-only">View details</span>
            </Button>
          </div>
        </TableCell>
      </TableRow>
    )
  }
)

HistoryTableRow.displayName = 'HistoryTableRow'

export function PurchaseHistoryList({
  purchases,
  isLoading,
  error,
  onDetailsClick,
  onApplyInvoice,
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
        <h3 className="text-lg font-semibold">{m['points.purchase_history_error_title']()}</h3>
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
        <h3 className="text-lg font-semibold">{m['points.purchase_history_empty_title']()}</h3>
        <p className="text-sm text-muted-foreground">
          You haven't purchased any points packages yet
        </p>
      </div>
    )
  }

  return (
    <div data-testid="purchase-history-list" className="rounded-md border">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{m['points.purchase_history_date']()}</TableHead>
            <TableHead>{m['points.purchase_history_package']()}</TableHead>
            <TableHead>{m['points.purchase_history_points']()}</TableHead>
            <TableHead>{m['points.purchase_history_amount']()}</TableHead>
            <TableHead>{m['points.purchase_history_provider']()}</TableHead>
            <TableHead className="text-right">{m['points.purchase_history_actions']()}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {purchases.map((purchase) => (
            <HistoryTableRow
              key={purchase.id}
              purchase={purchase}
              onDetailsClick={onDetailsClick}
              onApplyInvoice={onApplyInvoice}
            />
          ))}
        </TableBody>
      </Table>
    </div>
  )
}
