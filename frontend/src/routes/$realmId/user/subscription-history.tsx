import { useCallback, useState } from 'react'
import { createFileRoute, Link, useNavigate } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ListPagination, PageHeader } from '@/components/shared'
import { PurchaseHistoryList } from '@/components/purchase/purchase-history-list'
import { PurchaseDetailsDialog } from '@/components/purchase/purchase-details-dialog'
import {
  userFeatureAvailabilityQueryOptions,
  purchaseHistoryQueryOptions,
  requireUserFeature,
} from '@/data/query-options'
import { DEFAULT_PAGE_SIZE } from '@/lib/constants'
import type { PurchaseHistoryItem } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'
import { realmPath, useResolvedRealmContext } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/user/subscription-history')({
  beforeLoad: ({ context, params }) =>
    requireUserFeature(context.queryClient, (f) => f.user.pointsVisible, {
      to: '/$realmId/user/profile',
      params: { realmId: params.realmId },
    }),
  component: PurchaseRecordsRoute,
})

export function PurchaseRecordsRoute() {
  const realmContext = useResolvedRealmContext()
  const realmId = realmContext.realmId
  const navigate = useNavigate()
  const [purchaseHistoryPage, setPurchaseHistoryPage] = useState(1)
  const [selectedPurchase, setSelectedPurchase] = useState<PurchaseHistoryItem | null>(null)

  const { data: purchaseHistoryData, isLoading: purchaseHistoryLoading } = useQuery(
    purchaseHistoryQueryOptions(realmId, {
      page: purchaseHistoryPage,
      pageSize: DEFAULT_PAGE_SIZE,
    })
  )
  const { data: features } = useQuery(userFeatureAvailabilityQueryOptions)
  const invoicesVisible = features?.user.invoicesVisible === true
  const canPurchasePoints = features?.user.pointsVisible === true

  const handleDetailsClick = useCallback(
    (attemptId: string) => {
      const purchase = purchaseHistoryData?.items?.find((p) => p.attemptId === attemptId)
      if (purchase) setSelectedPurchase(purchase)
    },
    [purchaseHistoryData?.items]
  )

  const handleApplyInvoice = useCallback(
    (attemptId: string) => {
      navigate({
        to: realmPath(realmContext, '/user/invoices/new'),
        search: {
          paymentAttemptId: attemptId,
          returnTo: realmPath(realmContext, '/user/subscription-history'),
        },
      })
    },
    [realmContext, navigate]
  )

  return (
    <div className="space-y-6" data-testid="purchase-records-page">
      <div className="flex items-center justify-between gap-4">
        <PageHeader title={m['billing.purchase_records_page_title']()} />
        {canPurchasePoints && (
          <Button asChild data-testid="purchase-records-purchase-points-button">
            <Link to={realmPath(realmContext, '/user/purchase-points')}>
              <Plus className="mr-2 h-4 w-4" />
              {m['points.user_points_purchase_button']()}
            </Link>
          </Button>
        )}
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{m['billing.purchase_records_history_title']()}</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <PurchaseHistoryList
            purchases={purchaseHistoryData?.items || []}
            isLoading={purchaseHistoryLoading}
            onDetailsClick={handleDetailsClick}
            realmId={invoicesVisible ? realmId : undefined}
            onApplyInvoice={invoicesVisible ? handleApplyInvoice : undefined}
          />
          {purchaseHistoryData && purchaseHistoryData.total > 0 && (
            <ListPagination
              page={purchaseHistoryPage - 1}
              pageSize={DEFAULT_PAGE_SIZE}
              total={purchaseHistoryData.total}
              onPageChange={(page) => setPurchaseHistoryPage(page + 1)}
              testIdPrefix="purchase-records-pagination"
            />
          )}
        </CardContent>
      </Card>

      <PurchaseDetailsDialog
        purchase={selectedPurchase}
        open={selectedPurchase !== null}
        onOpenChange={(open) => {
          if (!open) setSelectedPurchase(null)
        }}
      />
    </div>
  )
}
