import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Input } from '@/components/ui/input'
import { History, Search } from 'lucide-react'
import { TransactionHistoryTable } from '../TransactionHistoryTable'
import { GrantPointsDialog } from '../grant-points-dialog'
import { pointsWalletsQueryOptions, pointsTransactionsQueryOptions } from '@/data/query-options'
import type { TransactionFilters } from '@/lib/schemas/points-forms'
import { DEFAULT_PAGE_SIZE } from '@/lib/constants'
import { PERMISSION } from '@/lib/constants/auth-constants'
import { PageHeader, ListPagination } from '@/components/shared'
import { usePermission } from '@/hooks/use-permission'

interface PointsWalletsPageProps {
  realmId: string
}

export function PointsWalletsPage({ realmId }: PointsWalletsPageProps) {
  // TODO: Migrate search/pagination/filter state to URL search params via parent route
  // (/$realmId/manage/points/wallets) for link sharing and refresh restoration.
  // Requires adding: selectedUserId, searchQuery, walletsPage, transactionsPage,
  // and transactionFilters to the parent route's validateSearch.

  const { hasPermission } = usePermission()
  const [grantDialogOpen, setGrantDialogOpen] = useState(false)

  // UI state
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState<string>('')

  // Filters
  const [transactionFilters, setTransactionFilters] = useState<TransactionFilters>({})

  // Pagination
  const [walletsPage, setWalletsPage] = useState(1)
  const [transactionsPage, setTransactionsPage] = useState(1)

  // Queries
  const { data: walletsData, isLoading: walletsLoading } = useQuery(
    pointsWalletsQueryOptions(realmId, {
      page: walletsPage,
      pageSize: DEFAULT_PAGE_SIZE,
      search: searchQuery,
    })
  )

  const { data: transactionsData, isLoading: transactionsLoading } = useQuery(
    pointsTransactionsQueryOptions(realmId, {
      userId: selectedUserId || undefined,
      page: transactionsPage,
      pageSize: DEFAULT_PAGE_SIZE,
      ...transactionFilters,
    })
  )

  function handleUserSelect(userId: string) {
    setSelectedUserId(userId)
    setTransactionsPage(1)
    setTransactionFilters({})
  }

  return (
    <>
      <div className="space-y-6" data-testid="points-wallets-page">
        <PageHeader
          title="Points Wallets"
          action={{
            label: 'Grant Points',
            onClick: () => setGrantDialogOpen(true),
            testId: 'grant-points-button',
            show: hasPermission(PERMISSION.POINTS_MANAGE),
          }}
        />

        {/* Points Wallets List */}
        <Card>
          <CardHeader>
            <div className="flex items-center justify-between">
              <CardTitle>Points Wallets</CardTitle>
              <div className="relative w-80">
                <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
                <Input
                  placeholder="Search by user ID..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-10"
                  data-testid="wallets-search-input"
                />
              </div>
            </div>
          </CardHeader>
          <CardContent>
            {walletsLoading ? (
              <div className="text-center py-8">Loading wallets...</div>
            ) : walletsData?.wallets && walletsData.wallets.length > 0 ? (
              <div className="space-y-2">
                {walletsData.wallets.map((wallet) => (
                  <div
                    key={wallet.id}
                    className={`p-4 border rounded-lg cursor-pointer hover:bg-muted/50 transition-colors ${
                      selectedUserId === wallet.userId ? 'bg-muted border-primary' : ''
                    }`}
                    onClick={() => handleUserSelect(wallet.userId)}
                    data-testid={`wallet-row-${wallet.userId}`}
                  >
                    <div className="flex items-center justify-between">
                      <div>
                        <div className="font-medium">{wallet.userId}</div>
                      </div>
                      <div className="text-right">
                        <div className="text-2xl font-bold">{wallet.balance.toLocaleString()}</div>
                        <Badge variant={wallet.status === 'active' ? 'default' : 'secondary'}>
                          {wallet.status}
                        </Badge>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-center py-8 text-muted-foreground">No points wallets found</div>
            )}
          </CardContent>
        </Card>

        {walletsData && walletsData.total > 0 && (
          <ListPagination
            page={walletsPage - 1}
            pageSize={DEFAULT_PAGE_SIZE}
            total={walletsData.total}
            onPageChange={(page) => setWalletsPage(page + 1)}
            testIdPrefix="wallets-pagination"
          />
        )}

        {/* Transaction History */}
        {selectedUserId && (
          <div className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <History className="h-4 w-4" />
                  Transaction History
                </CardTitle>
              </CardHeader>
              <CardContent>
                <TransactionHistoryTable
                  transactions={transactionsData?.transactions || []}
                  loading={transactionsLoading}
                  filters={transactionFilters}
                  admin={true}
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
          </div>
        )}
      </div>
      {grantDialogOpen && (
        <GrantPointsDialog
          open={grantDialogOpen}
          onOpenChange={setGrantDialogOpen}
          realmId={realmId}
        />
      )}
    </>
  )
}
