import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Coins, History, TrendingUp, TrendingDown, Wallet, Info, Plus } from 'lucide-react'
import { Link } from '@tanstack/react-router'
import { PointsBalanceCard } from './PointsBalanceCard'
import { TransactionHistoryTable } from './TransactionHistoryTable'
import { TransactionFilters } from './TransactionFilters'
import { PurchaseHistoryList } from '@/components/purchase/purchase-history-list'
import { PurchaseDetailsDialog } from '@/components/purchase/purchase-details-dialog'
import {
  pointsAccountQueryOptions,
  pointsTransactionsQueryOptions,
  pointsPackagePurchaseHistoryQueryOptions,
} from '@/data/query-options'
import { DEFAULT_PAGE_SIZE } from '@/lib/constants'
import type { TransactionFilters as TransactionFiltersType } from '@/lib/schemas/points-forms'

interface UserPointsPageProps {
  realmId: string
  userId: string
}

export function UserPointsPage({ realmId, userId }: UserPointsPageProps) {
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

  return (
    <div className="space-y-6" data-testid="user-points-page">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-3xl font-bold">My Points</h1>
          <p className="text-muted-foreground">View your points balance and transaction history</p>
        </div>
        <Link to="/$realmId/user/purchase-points" params={{ realmId }}>
          <Button data-testid="purchase-points-button">
            <Plus className="mr-2 h-4 w-4" />
            Purchase Points
          </Button>
        </Link>
      </div>

      {/* Balance Card */}
      <PointsBalanceCard account={account || null} loading={accountLoading} />

      {/* Points Description */}
      <Card className="bg-blue-50 border-blue-200">
        <CardContent className="p-6">
          <div className="flex items-start gap-4">
            <Info className="h-6 w-6 text-blue-600 mt-0.5" />
            <div>
              <h3 className="font-semibold text-blue-900">About Points</h3>
              <p className="text-sm text-blue-700 mt-1" data-testid="points-description-text">
                积分是系统的虚拟货币，可用于消耗第三方应用的服务
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Quick Stats */}
      {account && (
        <div className="grid gap-4 md:grid-cols-3">
          <Card>
            <CardContent className="p-6">
              <div className="flex items-center gap-4">
                <div className="p-3 bg-primary/10 rounded-lg">
                  <Wallet className="h-6 w-6 text-primary" />
                </div>
                <div>
                  <div className="text-sm text-muted-foreground">Current Balance</div>
                  <div className="text-2xl font-bold">{account.balance.toLocaleString()}</div>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6">
              <div className="flex items-center gap-4">
                <div className="p-3 bg-green-500/10 rounded-lg">
                  <TrendingUp className="h-6 w-6 text-green-600" />
                </div>
                <div>
                  <div className="text-sm text-muted-foreground">Total Earned</div>
                  <div className="text-2xl font-bold text-green-600">
                    {account.totalRecharged.toLocaleString()}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-6">
              <div className="flex items-center gap-4">
                <div className="p-3 bg-red-500/10 rounded-lg">
                  <TrendingDown className="h-6 w-6 text-red-600" />
                </div>
                <div>
                  <div className="text-sm text-muted-foreground">Total Spent</div>
                  <div className="text-2xl font-bold text-red-600">
                    {account.totalConsumed.toLocaleString()}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

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
                pagination={{
                  page: transactionsPage,
                  pageSize: DEFAULT_PAGE_SIZE,
                  total: transactionsData?.total || 0,
                }}
                onPaginationChange={(pagination) => setTransactionsPage(pagination.page)}
                admin={false}
              />
            </CardContent>
          </Card>
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
                pagination={
                  purchaseHistoryData
                    ? {
                        page: purchaseHistoryPage,
                        pageSize: DEFAULT_PAGE_SIZE,
                        total: purchaseHistoryData?.purchases?.length || 0,
                        totalPages: Math.ceil(
                          (purchaseHistoryData?.purchases?.length || 0) / DEFAULT_PAGE_SIZE
                        ),
                      }
                    : undefined
                }
                onPageChange={setPurchaseHistoryPage}
                onDetailsClick={(purchaseId) => {
                  setSelectedPurchaseId(purchaseId)
                }}
              />
            </CardContent>
          </Card>
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

      {/* Info Banner */}
      {account && account.balance < 100 && (
        <Card className="bg-amber-50 border-amber-200">
          <CardContent className="p-6">
            <div className="flex items-start gap-4">
              <Coins className="h-6 w-6 text-amber-600 mt-0.5" />
              <div>
                <h3 className="font-semibold text-amber-900">Low Points Balance</h3>
                <p className="text-sm text-amber-700 mt-1">
                  You have less than 100 points remaining. Consider subscribing to a plan to earn
                  more points and avoid service interruptions.
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  )
}
