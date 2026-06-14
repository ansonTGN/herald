import { useState, useCallback } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Coins, History, Plus } from 'lucide-react'
import { Link, useNavigate } from '@tanstack/react-router'
import { PointsBalanceCard } from './PointsBalanceCard'
import { TransactionHistoryTable } from './TransactionHistoryTable'
import { TransactionFilters } from './TransactionFilters'
import { PurchaseHistoryList } from '@/components/purchase/purchase-history-list'
import { PurchaseDetailsDialog } from '@/components/purchase/purchase-details-dialog'
import {
  pointsWalletQueryOptions,
  pointsTransactionsQueryOptions,
  purchaseHistoryQueryOptions,
  featureAvailabilityQueryOptions,
} from '@/data/query-options'
import { DEFAULT_PAGE_SIZE } from '@/lib/constants'
import type { TransactionFilters as TransactionFiltersType } from '@/lib/schemas/points-forms'
import type { PurchaseHistoryItemDto } from '@/lib/api-generated'
import { ListPagination } from '@/components/shared'
import { m } from '@/paraglide/messages'

interface UserPointsPageProps {
  realmId: string
  userId: string
}

export function UserPointsPage({ realmId, userId }: UserPointsPageProps) {
  const navigate = useNavigate()
  // TODO: Migrate pagination/filter state to URL search params via parent route
  // (/$realmId/user/points) for link sharing and refresh restoration.
  // Requires adding: tab, transactionsPage, transactionFilters (type/startTime/endTime),
  // purchaseHistoryPage to the parent route's validateSearch.
  const [transactionsPage, setTransactionsPage] = useState(1)
  const [transactionFilters, setTransactionFilters] = useState<TransactionFiltersType>({})
  const [purchaseHistoryPage, setPurchaseHistoryPage] = useState(1)
  const [selectedPurchase, setSelectedPurchase] = useState<PurchaseHistoryItemDto | null>(null)

  const { data: wallet, isLoading: walletLoading } = useQuery(
    pointsWalletQueryOptions(realmId, userId)
  )

  const { data: transactionsData, isLoading: transactionsLoading } = useQuery(
    pointsTransactionsQueryOptions(realmId, {
      userId: userId,
      page: transactionsPage,
      pageSize: DEFAULT_PAGE_SIZE,
      ...transactionFilters,
    })
  )

  const { data: purchaseHistoryData, isLoading: purchaseHistoryLoading } = useQuery(
    purchaseHistoryQueryOptions(realmId, {
      page: purchaseHistoryPage,
      pageSize: DEFAULT_PAGE_SIZE,
    })
  )
  const { data: features } = useQuery(featureAvailabilityQueryOptions(realmId))
  const invoicesVisible = features?.user.invoicesVisible === true

  // Dialog open state is derived from selectedPurchase
  const purchaseDetailsOpen = selectedPurchase !== null

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
        to: '/$realmId/user/invoices/new',
        params: { realmId },
        search: {
          paymentAttemptId: attemptId,
          returnTo: `/${realmId}/user/points`,
        },
      })
    },
    [realmId, navigate]
  )

  return (
    <div className="space-y-6" data-testid="user-points-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{m['points.user_points_page_title']()}</h1>
        {features?.user.pointsPurchaseVisible === true && (
          <Link to="/$realmId/user/purchase-points" params={{ realmId }}>
            <Button data-testid="purchase-points-button">
              <Plus className="mr-2 h-4 w-4" />
              {m['points.user_points_purchase_button']()}
            </Button>
          </Link>
        )}
      </div>

      {/* Balance Card */}
      <PointsBalanceCard account={wallet ?? null} loading={walletLoading} />

      {/* Transaction History and Purchase History Tabs */}
      <Tabs defaultValue="transactions" className="space-y-4" data-testid="points-page-tabs">
        <TabsList>
          <TabsTrigger value="transactions" data-testid="points-tab-transactions">
            {m['points.user_points_transaction_tab']()}
          </TabsTrigger>
          <TabsTrigger value="purchase-history" data-testid="points-tab-purchase-history">
            {m['points.user_points_purchase_history_tab']()}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="transactions" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <History className="h-4 w-4" />
                {m['points.user_points_transaction_history']()}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <TransactionFilters
                filters={transactionFilters}
                onChange={(filters) => {
                  setTransactionFilters(filters)
                  setTransactionsPage(1) // Reset to first page when filters change
                }}
                onClear={() => {
                  setTransactionFilters({})
                  setTransactionsPage(1)
                }}
                admin={false}
                loading={transactionsLoading}
              />
              <TransactionHistoryTable
                transactions={transactionsData?.transactions || []}
                loading={transactionsLoading}
                filters={transactionFilters}
                admin={false}
              />
            </CardContent>
          </Card>
          {transactionsData && transactionsData.total > 0 && (
            <ListPagination
              page={transactionsPage - 1}
              pageSize={DEFAULT_PAGE_SIZE}
              total={transactionsData.total}
              onPageChange={(page) => setTransactionsPage(page + 1)}
              testIdPrefix="transaction-pagination"
            />
          )}
        </TabsContent>

        <TabsContent value="purchase-history" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Coins className="h-4 w-4" />
                {m['points.user_points_purchase_history']()}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <PurchaseHistoryList
                purchases={purchaseHistoryData?.items || []}
                isLoading={purchaseHistoryLoading}
                onDetailsClick={handleDetailsClick}
                realmId={invoicesVisible ? realmId : undefined}
                onApplyInvoice={invoicesVisible ? handleApplyInvoice : undefined}
              />
            </CardContent>
          </Card>
          {purchaseHistoryData && purchaseHistoryData.total > 0 && (
            <ListPagination
              page={purchaseHistoryPage - 1}
              pageSize={DEFAULT_PAGE_SIZE}
              total={purchaseHistoryData.total}
              onPageChange={(page) => setPurchaseHistoryPage(page + 1)}
              testIdPrefix="purchase-pagination"
            />
          )}
        </TabsContent>
      </Tabs>

      {/* Purchase Details Dialog */}
      <PurchaseDetailsDialog
        purchase={selectedPurchase}
        open={purchaseDetailsOpen}
        onOpenChange={(open) => {
          if (!open) setSelectedPurchase(null)
        }}
      />
    </div>
  )
}
