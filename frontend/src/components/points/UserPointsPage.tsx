import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Coins, History, Plus } from 'lucide-react'
import { Link, useNavigate } from '@tanstack/react-router'
import { TransactionHistoryTable } from './TransactionHistoryTable'
import { TransactionFilters } from './TransactionFilters'
import { PurchaseHistoryList } from '@/components/purchase/purchase-history-list'
import { PurchaseDetailsDialog } from '@/components/purchase/purchase-details-dialog'
import {
  pointsAccountQueryOptions,
  pointsTransactionsQueryOptions,
  pointsPackagePurchaseHistoryQueryOptions,
  featureAvailabilityQueryOptions,
} from '@/data/query-options'
import { DEFAULT_PAGE_SIZE } from '@/lib/constants'
import type { TransactionFilters as TransactionFiltersType } from '@/lib/schemas/points-forms'
import { ListPagination } from '@/components/shared'

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
  const [selectedPurchaseId, setSelectedPurchaseId] = useState<string | null>(null)

  // Dialog open state is derived from selectedPurchaseId
  const purchaseDetailsOpen = selectedPurchaseId !== null

  const { data: account, isLoading: accountLoading } = useQuery(
    pointsAccountQueryOptions(realmId, userId)
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
    pointsPackagePurchaseHistoryQueryOptions(realmId, {
      userId: userId,
      page: purchaseHistoryPage,
      pageSize: DEFAULT_PAGE_SIZE,
    })
  )
  const { data: features } = useQuery(featureAvailabilityQueryOptions(realmId))
  const invoicesVisible = features?.user.invoicesVisible === true

  return (
    <div className="space-y-6" data-testid="user-points-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold">My Points</h1>
          {account && !accountLoading && (
            <span className="text-2xl font-bold text-emerald-600" data-testid="points-balance">
              {account.balance.toLocaleString()}{' '}
              <span className="text-sm font-normal text-muted-foreground">{account.unit}</span>
            </span>
          )}
          {accountLoading && (
            <span
              className="text-2xl font-bold text-emerald-600 animate-pulse"
              data-testid="points-balance"
            >
              ---
            </span>
          )}
        </div>
        {features?.user.pointsPurchaseVisible !== false && (
          <Link to="/$realmId/user/purchase-points" params={{ realmId }}>
            <Button data-testid="purchase-points-button">
              <Plus className="mr-2 h-4 w-4" />
              Purchase Points
            </Button>
          </Link>
        )}
      </div>

      {/* Transaction History and Purchase History Tabs */}
      <Tabs defaultValue="transactions" className="space-y-4" data-testid="points-page-tabs">
        <TabsList>
          <TabsTrigger value="transactions" data-testid="points-tab-transactions">
            Transaction History
          </TabsTrigger>
          <TabsTrigger value="purchase-history" data-testid="points-tab-purchase-history">
            Purchase History
          </TabsTrigger>
        </TabsList>

        <TabsContent value="transactions" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <History className="h-4 w-4" />
                Transaction History
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
                Points Package Purchase History
              </CardTitle>
            </CardHeader>
            <CardContent>
              <PurchaseHistoryList
                purchases={purchaseHistoryData?.purchases || []}
                isLoading={purchaseHistoryLoading}
                onDetailsClick={(purchaseId) => {
                  setSelectedPurchaseId(purchaseId)
                }}
                onApplyInvoice={
                  invoicesVisible
                    ? (paymentAttemptId) => {
                        navigate({
                          to: '/$realmId/user/invoices/new',
                          params: { realmId },
                          search: {
                            paymentAttemptId,
                            returnTo: `/${realmId}/user/points`,
                          },
                        })
                      }
                    : undefined
                }
              />
            </CardContent>
          </Card>
          {purchaseHistoryData &&
            purchaseHistoryData.purchases &&
            purchaseHistoryData.purchases.length > 0 && (
              <ListPagination
                page={purchaseHistoryPage - 1}
                pageSize={DEFAULT_PAGE_SIZE}
                total={purchaseHistoryData.purchases.length}
                onPageChange={(page) => setPurchaseHistoryPage(page + 1)}
                testIdPrefix="purchase-pagination"
              />
            )}
        </TabsContent>
      </Tabs>

      {/* Purchase Details Dialog */}
      {selectedPurchaseId && (
        <PurchaseDetailsDialog
          purchaseId={selectedPurchaseId}
          realmId={realmId}
          open={purchaseDetailsOpen}
          onOpenChange={(open) => {
            if (!open) setSelectedPurchaseId(null)
          }}
        />
      )}
    </div>
  )
}
