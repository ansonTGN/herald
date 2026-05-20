import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { SubscriptionHistoryList } from '@/components/billing/subscription-history-list'
import { SubscriptionHistoryFilter } from '@/components/billing/subscription-history-filter'
import {
  subscriptionPlansQueryOptions,
  globalSubscriptionHistoryQueryOptions,
  requireFeature,
} from '@/data/query-options'
import type { HistoryFilters, SubscriptionHistoryEventWithUser } from '@/types/billing'
import { toast } from 'sonner'
import { PageHeader, ListPagination } from '@/components/shared'

export const Route = createFileRoute('/$realmId/manage/subscription-history')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.admin.subscriptionHistoryVisible, {
      to: '/$realmId/manage/billing',
      params: { realmId: params.realmId },
      search: { status: 'all' },
    }),
  component: SubscriptionHistoryRoute,
})

function SubscriptionHistoryRoute() {
  const { realmId } = Route.useParams()

  // Filter state
  const [filters, setFilters] = useState<HistoryFilters>({
    sortBy: 'timestamp',
    sortOrder: 'desc',
  })
  const [page, setPage] = useState(1)
  const pageSize = 20

  // Query plans for filter dropdown
  const { data: plansData } = useQuery(subscriptionPlansQueryOptions(realmId))
  const plans = plansData?.items ?? []

  // Query subscription history
  const {
    data: historyData,
    isLoading,
    error,
  } = useQuery(globalSubscriptionHistoryQueryOptions(realmId, filters, page, pageSize))

  // Handle filter changes
  function handleFiltersChange(newFilters: HistoryFilters) {
    setFilters(newFilters)
    setPage(1) // Reset to first page on filter change
  }

  // Handle filter reset
  function handleResetFilters() {
    setFilters({
      sortBy: 'timestamp',
      sortOrder: 'desc',
    })
    setPage(1)
  }

  // Handle page changes
  function handlePageChange(newPage: number) {
    setPage(newPage)
  }

  // Handle sort changes
  function handleSortChange(sortBy: string) {
    setFilters((prev) => ({
      ...prev,
      sortBy,
      sortOrder: prev.sortBy === sortBy && prev.sortOrder === 'desc' ? 'asc' : 'desc',
    }))
    setPage(1)
  }

  // Handle event click
  function handleEventClick(event: SubscriptionHistoryEventWithUser) {
    toast.info(`Event details: ${event.eventType}`, {
      description: `Timestamp: ${new Date(event.timestamp).toLocaleString()}`,
    })
  }

  if (error) {
    return (
      <div className="space-y-6" data-testid="subscription-history-page">
        <Card className="border-destructive">
          <CardContent className="p-6">
            <p className="text-destructive">
              Failed to load subscription history:{' '}
              {error instanceof Error ? error.message : 'Unknown error'}
            </p>
            <Button variant="outline" className="mt-4" onClick={() => window.location.reload()}>
              Retry
            </Button>
          </CardContent>
        </Card>
      </div>
    )
  }

  if (isLoading) {
    return (
      <div className="space-y-6" data-testid="subscription-history-page">
        <div className="flex items-center justify-center py-12" data-testid="page-loading">
          <div className="text-center">
            <div className="inline-block h-8 w-8 animate-spin rounded-full border-4 border-solid border-primary border-r-transparent" />
            <p className="mt-4 text-muted-foreground">Loading subscription history...</p>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6" data-testid="subscription-history-page">
      <PageHeader title="Subscription History" />

      {/* Filters */}
      <SubscriptionHistoryFilter
        filters={filters}
        onFiltersChange={handleFiltersChange}
        onReset={handleResetFilters}
        plans={plans?.map((plan) => ({ id: plan.id, name: plan.title || plan.id }))}
        loading={isLoading}
      />

      {/* History List */}
      <Card>
        <CardHeader>
          <CardTitle>History Events</CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <SubscriptionHistoryList
            events={historyData?.events || []}
            loading={isLoading}
            onSortChange={handleSortChange}
            onEventClick={handleEventClick}
          />
        </CardContent>
      </Card>

      {historyData?.pagination && historyData.pagination.totalCount > 0 && (
        <ListPagination
          page={historyData.pagination.page - 1}
          pageSize={pageSize}
          total={historyData.pagination.totalCount}
          onPageChange={(newPage) => handlePageChange(newPage + 1)}
          testIdPrefix="subscription-history-pagination"
        />
      )}
    </div>
  )
}
