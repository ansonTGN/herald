import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { History } from 'lucide-react'
import { PointsBalanceCard } from './PointsBalanceCard'
import { TransactionHistoryTable } from './TransactionHistoryTable'
import { TransactionFilters } from './TransactionFilters'
import { pointsWalletQueryOptions, pointsTransactionsQueryOptions } from '@/data/query-options'
import { DEFAULT_PAGE_SIZE } from '@/lib/constants'
import type { TransactionFilters as TransactionFiltersType } from '@/lib/schemas/points-forms'
import { m } from '@/paraglide/messages'

interface UserPointsPageProps {
  realmId: string
  userId: string
}

const MAX_VISIBLE_TRANSACTIONS = 1000

export function UserPointsPage({ realmId, userId }: UserPointsPageProps) {
  // TODO: Migrate pagination/filter state to URL search params via parent route
  // (/$realmId/user/points) for link sharing and refresh restoration.
  // Requires adding: loadedPages and transactionFilters (type/startTime/endTime)
  // to the parent route's validateSearch.
  // loadedPages counts how many "Load More" windows have been expanded; the
  // server returns the latest N transactions in a single growing page, so we
  // never accumulate fetched pages in component state (which would need effects).
  const [loadedPages, setLoadedPages] = useState(1)
  const [transactionFilters, setTransactionFilters] = useState<TransactionFiltersType>({})

  const { data: wallet, isLoading: walletLoading } = useQuery(
    pointsWalletQueryOptions(realmId, userId)
  )

  const requestedPageSize = Math.min(loadedPages * DEFAULT_PAGE_SIZE, MAX_VISIBLE_TRANSACTIONS)
  const { data: transactionsData, isLoading: transactionsLoading } = useQuery(
    pointsTransactionsQueryOptions(realmId, {
      userId,
      page: 1,
      pageSize: requestedPageSize,
      ...transactionFilters,
    })
  )

  const transactions = transactionsData?.transactions ?? []
  const reachedLimit = transactions.length >= MAX_VISIBLE_TRANSACTIONS
  const canLoadMore =
    !transactionsLoading && transactions.length >= requestedPageSize && !reachedLimit

  function handleFiltersChange(filters: TransactionFiltersType) {
    setTransactionFilters(filters)
    setLoadedPages(1)
  }

  function handleFiltersClear() {
    setTransactionFilters({})
    setLoadedPages(1)
  }

  return (
    <div className="space-y-6" data-testid="user-points-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{m['points.user_points_page_title']()}</h1>
      </div>

      {/* Balance Card */}
      <PointsBalanceCard account={wallet ?? null} loading={walletLoading} />

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
            onChange={handleFiltersChange}
            onClear={handleFiltersClear}
            admin={false}
            loading={transactionsLoading}
          />
          <TransactionHistoryTable
            transactions={transactions}
            loading={transactionsLoading && loadedPages === 1}
            filters={transactionFilters}
            admin={false}
          />
          {reachedLimit && (
            <p className="text-center text-sm text-muted-foreground">
              {m['points.transaction_load_limit_reached']({
                count: MAX_VISIBLE_TRANSACTIONS.toLocaleString(),
              })}
            </p>
          )}
          {canLoadMore && (
            <div className="flex justify-center">
              <Button
                variant="outline"
                onClick={() => setLoadedPages((pages) => pages + 1)}
                disabled={transactionsLoading}
                data-testid="transaction-load-more"
              >
                {m['points.transaction_load_more']()}
              </Button>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
