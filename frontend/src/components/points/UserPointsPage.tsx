import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { History } from 'lucide-react'
import { PointsBalanceCard } from './PointsBalanceCard'
import { TransactionHistoryTable } from './TransactionHistoryTable'
import { TransactionFilters } from './TransactionFilters'
import { deriveUserPointsView } from './user-points-view'
import { walletsByBucketQueryOptions, pointsTransactionsQueryOptions } from '@/data/query-options'
import { useEnabledBuckets } from '@/data/use-buckets'
import { DEFAULT_PAGE_SIZE } from '@/lib/constants'
import type { TransactionFilters as TransactionFiltersType } from '@/lib/schemas/points-forms'
import { m } from '@/paraglide/messages'

interface UserPointsPageProps {
  realmId: string
  userId: string
  /**
   * URL-synced `bucketId` transaction filter (design §4.2.3 `?bucketId=`).
   * Source of truth is the `/$realmId/user/points` route `validateSearch`
   * (FE-D06); this component mirrors it into `transactionFilters` and reports
   * user-driven changes back via {@link onBucketIdChange} so the URL stays
   * shareable. `undefined` means "all buckets".
   */
  bucketId?: string
  onBucketIdChange?: (bucketId: string | undefined) => void
}

const MAX_VISIBLE_TRANSACTIONS = 1000

export function UserPointsPage({
  realmId,
  userId,
  bucketId,
  onBucketIdChange,
}: UserPointsPageProps) {
  // TODO: Migrate pagination/filter state to URL search params via parent route
  // (/$realmId/user/points) for link sharing and refresh restoration.
  // `bucketId` is already URL-synced (FE-D06); the remaining ephemeral filters
  // (type/startTime/endTime) are intentionally local state.
  // loadedPages counts how many "Load More" windows have been expanded; the
  // server returns the latest N transactions in a single growing page, so we
  // never accumulate fetched pages in component state (which would need effects).
  const [loadedPages, setLoadedPages] = useState(1)
  // Ephemeral transaction filters (type/dates). `bucketId` is intentionally
  // NOT stored here: it is URL-synced (FE-D06) and merged in below so external
  // URL changes (e.g. opening a shared `?bucketId=` link) reflect immediately.
  const [transactionFilters, setTransactionFilters] = useState<TransactionFiltersType>({})

  // FE-D01 LOUD DEVIATION: `listWallets` is realm-wide and `points.view`-gated
  // with NO userId filter, so `data.items` contains ALL realm users' rows and
  // `data.crossBucketTotal` is the realm cross-user total (not the current
  // user's). We client-filter by `userId` and recompute this user's total via
  // `deriveUserPointsView`. Do NOT use the response `crossBucketTotal` here.
  const { data: walletsData, isLoading: walletsLoading } = useQuery(
    walletsByBucketQueryOptions(realmId)
  )

  // Enabled buckets feed the user-facing Bucket Select (disabled buckets are
  // not selectable for filtering) and the Bucket column lookup.
  const { buckets: enabledBuckets } = useEnabledBuckets(realmId)

  // Effective filters = ephemeral (type/dates) + URL-derived bucketId. The
  // URL wins for `bucketId` so the page always matches the shareable URL.
  const effectiveFilters: TransactionFiltersType = {
    ...transactionFilters,
    bucketId,
  }

  const requestedPageSize = Math.min(loadedPages * DEFAULT_PAGE_SIZE, MAX_VISIBLE_TRANSACTIONS)
  const { data: transactionsData, isLoading: transactionsLoading } = useQuery(
    pointsTransactionsQueryOptions(realmId, {
      userId,
      page: 1,
      pageSize: requestedPageSize,
      ...effectiveFilters,
    })
  )

  const { showTotalBar, cards, crossBucketTotal } = deriveUserPointsView(
    walletsData?.items ?? [],
    userId
  )

  const transactions = transactionsData?.transactions ?? []
  const reachedLimit = transactions.length >= MAX_VISIBLE_TRANSACTIONS
  const canLoadMore =
    !transactionsLoading && transactions.length >= requestedPageSize && !reachedLimit

  function handleFiltersChange(filters: TransactionFiltersType) {
    // `bucketId` is URL-owned; strip it before storing ephemeral filters so
    // the URL remains the single source of truth for the bucket dimension.
    const ephemeral: TransactionFiltersType = {
      transactionType: filters.transactionType,
      startTime: filters.startTime,
      endTime: filters.endTime,
      clientAppId: filters.clientAppId,
    }
    setTransactionFilters(ephemeral)
    setLoadedPages(1)
    // Mirror the shareable bucket dimension back to the URL (FE-D06).
    onBucketIdChange?.(filters.bucketId)
  }

  function handleFiltersClear() {
    setTransactionFilters({})
    setLoadedPages(1)
    onBucketIdChange?.(undefined)
  }

  return (
    <div className="space-y-6" data-testid="user-points-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{m['points.user_points_page_title']()}</h1>
      </div>

      {/* Cross-bucket total bar — only when the user holds >= 2 buckets */}
      {showTotalBar && (
        <Card data-testid="user-points-cross-bucket-total">
          <CardContent className="py-4">
            <div className="flex items-center justify-between">
              <span className="text-sm text-muted-foreground">
                {m['points.cross_bucket_total']()}
              </span>
              <span className="text-2xl font-bold">{crossBucketTotal.toLocaleString()}</span>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Bucket card stack / empty state */}
      {walletsLoading ? (
        <div className="space-y-4">
          <PointsBalanceCard
            card={{
              bucketId: '',
              name: null,
              enabled: null,
              bucketTotal: 0,
              balancesByType: {
                subscription: 0,
                topup: 0,
                registration: 0,
                freePeriodic: 0,
                granted: 0,
              },
            }}
            loading
          />
        </div>
      ) : cards.length === 0 ? (
        <Card data-testid="points-balance-empty">
          <CardContent className="py-8 text-center text-muted-foreground">
            {m['points.bucket_card_empty']()}
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-4">
          {cards.map((card) => (
            <PointsBalanceCard key={card.bucketId || 'unnamed'} card={card} />
          ))}
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <History className="h-4 w-4" />
            {m['points.user_points_transaction_history']()}
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <TransactionFilters
            filters={effectiveFilters}
            onChange={handleFiltersChange}
            onClear={handleFiltersClear}
            buckets={enabledBuckets}
            admin={false}
            loading={transactionsLoading}
          />
          <TransactionHistoryTable
            transactions={transactions}
            loading={transactionsLoading && loadedPages === 1}
            filters={effectiveFilters}
            buckets={enabledBuckets}
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
