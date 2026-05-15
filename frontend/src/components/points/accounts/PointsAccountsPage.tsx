import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { History, Search } from 'lucide-react'
import { PointsBalanceCard } from '../PointsBalanceCard'
import { TransactionHistoryTable } from '../TransactionHistoryTable'
import {
  pointsAccountsQueryOptions,
  pointsAccountQueryOptions,
  pointsTransactionsQueryOptions,
} from '@/data/query-options'
import type { TransactionFilters } from '@/lib/schemas/points-forms'
import { DEFAULT_PAGE_SIZE } from '@/lib/constants'
import { PageHeader } from '@/components/shared'

type PointsAccountListItem = {
  id: string
  userId: string
  userName?: string
  userEmail?: string
  realmId: string
  balance: number
  totalRecharged: number
  totalConsumed: number
  status: string
  createdAt: string
  updatedAt: string
  unit: string
}

interface PointsAccountsPageProps {
  realmId: string
}

export function PointsAccountsPage({ realmId }: PointsAccountsPageProps) {
  // TODO: Migrate search/pagination/filter state to URL search params via parent route
  // (/$realmId/manage/points/accounts) for link sharing and refresh restoration.
  // Requires adding: selectedUserId, searchQuery, accountsPage, transactionsPage,
  // and transactionFilters to the parent route's validateSearch.
  // UI state
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState<string>('')

  // Filters
  const [transactionFilters, setTransactionFilters] = useState<TransactionFilters>({})

  // Pagination
  const [accountsPage, setAccountsPage] = useState(1)
  const [transactionsPage, setTransactionsPage] = useState(1)

  // Queries
  const { data: accountsData, isLoading: accountsLoading } = useQuery(
    pointsAccountsQueryOptions(realmId, {
      page: accountsPage,
      pageSize: DEFAULT_PAGE_SIZE,
      search: searchQuery,
    })
  )

  const { data: selectedAccount, isLoading: accountLoading } = useQuery({
    ...pointsAccountQueryOptions(realmId, selectedUserId || ''),
    enabled: !!selectedUserId,
  })

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
    <div className="space-y-6" data-testid="points-accounts-page">
      <PageHeader
        title="User Accounts"
        description="Manage user points accounts and view transaction history"
      />

      {/* User Accounts List */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>User Points Accounts</CardTitle>
            <div className="relative w-80">
              <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
              <Input
                placeholder="Search by name, email, or user ID..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-10"
                data-testid="accounts-search-input"
              />
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {accountsLoading ? (
            <div className="text-center py-8">Loading accounts...</div>
          ) : accountsData?.accounts && accountsData.accounts.length > 0 ? (
            <div className="space-y-2">
              {accountsData.accounts.map((account: PointsAccountListItem) => (
                <div
                  key={account.id}
                  className={`p-4 border rounded-lg cursor-pointer hover:bg-muted/50 transition-colors ${
                    selectedUserId === account.userId ? 'bg-muted border-primary' : ''
                  }`}
                  onClick={() => handleUserSelect(account.userId)}
                  data-testid={`account-row-${account.userId}`}
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <div className="font-medium">{account.userName || account.userId}</div>
                      <div className="text-sm text-muted-foreground">
                        {account.userEmail || 'No email'}
                      </div>
                    </div>
                    <div className="text-right">
                      <div className="text-2xl font-bold">{account.balance.toLocaleString()}</div>
                      <Badge variant={account.status === 'active' ? 'default' : 'secondary'}>
                        {account.status}
                      </Badge>
                    </div>
                  </div>
                </div>
              ))}
              {/* Pagination */}
              {accountsData.total && accountsData.total > DEFAULT_PAGE_SIZE && (
                <div className="flex justify-center gap-2 mt-4">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setAccountsPage((p) => Math.max(1, p - 1))}
                    disabled={accountsPage === 1}
                  >
                    Previous
                  </Button>
                  <span className="px-3 py-1">
                    Page {accountsPage} of {Math.ceil(accountsData.total / DEFAULT_PAGE_SIZE)}
                  </span>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() =>
                      setAccountsPage((p) =>
                        p < Math.ceil(accountsData.total / DEFAULT_PAGE_SIZE) ? p + 1 : p
                      )
                    }
                    disabled={
                      accountsPage >= Math.ceil((accountsData.total || 0) / DEFAULT_PAGE_SIZE)
                    }
                  >
                    Next
                  </Button>
                </div>
              )}
            </div>
          ) : (
            <div className="text-center py-8 text-muted-foreground">No points accounts found</div>
          )}
        </CardContent>
      </Card>

      {/* Selected User Details */}
      {selectedUserId && (
        <div className="grid gap-6 md:grid-cols-2">
          {/* Balance Card */}
          <div>
            <PointsBalanceCard account={selectedAccount || null} loading={accountLoading} />
          </div>

          {/* Transaction History */}
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
                pagination={{
                  page: transactionsPage,
                  pageSize: DEFAULT_PAGE_SIZE,
                  total: transactionsData?.total || 0,
                }}
                onPaginationChange={(pagination) => setTransactionsPage(pagination.page)}
                admin={true}
              />
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  )
}
