import { m } from '@/paraglide/messages'
import { useQuery } from '@tanstack/react-query'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { format } from 'date-fns'
import { CreditCard, Package } from 'lucide-react'
import { pointsPackagePurchaseDetailsQueryOptions } from '@/data/query-options'
import { Skeleton } from '@/components/ui/skeleton'

interface PurchaseDetailsDialogProps {
  purchaseId: string
  realmId: string
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function PurchaseDetailsDialog({
  purchaseId,
  realmId,
  open,
  onOpenChange,
}: PurchaseDetailsDialogProps) {
  const { data: details, isLoading } = useQuery(
    pointsPackagePurchaseDetailsQueryOptions(realmId, purchaseId)
  )

  if (!open) return null

  return (
    <Dialog open={open} onOpenChange={onOpenChange} data-testid="purchase-details-dialog">
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{m['points.purchase_details_title']()}</DialogTitle>
          <DialogDescription>
            Detailed information about your points package purchase
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="space-y-4">
            <Skeleton className="h-20 w-full" />
            <Skeleton className="h-20 w-full" />
            <Skeleton className="h-20 w-full" />
          </div>
        ) : details ? (
          <div className="space-y-6">
            {/* Package Information */}
            <div className="rounded-lg border p-4" data-testid="purchase-details-package-info">
              <div className="mb-3 flex items-center gap-2">
                <Package className="h-5 w-5 text-primary" />
                <h3 className="font-semibold">{m['points.purchase_details_package_info']()}</h3>
              </div>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Package ID</span>
                  <span className="font-mono text-xs">{details.pointsPackageId}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Points</span>
                  <span className="font-medium">{details.points.toLocaleString()}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Amount</span>
                  <span className="font-medium">
                    {details.amount.toFixed(2)} {details.currency}
                  </span>
                </div>
              </div>
            </div>

            {/* Payment Information */}
            <div className="rounded-lg border p-4" data-testid="purchase-details-payment-info">
              <div className="mb-3 flex items-center gap-2">
                <CreditCard className="h-5 w-5 text-primary" />
                <h3 className="font-semibold">{m['points.purchase_details_payment_info']()}</h3>
              </div>
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Provider</span>
                  <span className="font-medium">{details.paymentProvider}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Purchase ID</span>
                  <span className="font-mono text-xs">{details.id}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Purchased</span>
                  <span className="font-medium">{format(new Date(details.createdAt), 'PPp')}</span>
                </div>
                {details.pointsTransactionId && (
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">Transaction ID</span>
                    <span className="font-mono text-xs">{details.pointsTransactionId}</span>
                  </div>
                )}
              </div>
            </div>
          </div>
        ) : (
          <div className="py-8 text-center text-sm text-muted-foreground">
            Failed to load purchase details
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
