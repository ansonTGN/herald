import { useState } from 'react'
import { createFileRoute, Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, Info } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { SubscriptionHistoryList } from '@/components/billing/subscription-history-list'
import { SubscriptionHistoryFilter } from '@/components/billing/subscription-history-filter'
import {
  billingPlansQueryOptions,
  globalSubscriptionHistoryQueryOptions,
} from '@/data/query-options'
import type { HistoryFilters, SubscriptionHistoryEventWithUser } from '@/types/billing'
import { toast } from 'sonner'

export const Route = createFileRoute('/$realmId/manage/subscription-history')({
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
  const { data: plansData } = useQuery(billingPlansQueryOptions(realmId))
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
      <div className="container mx-auto px-4 py-8" data-testid="subscription-history-page">
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
      <div className="container mx-auto px-4 py-8" data-testid="subscription-history-page">
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
    <div className="container mx-auto space-y-6 px-4 py-8" data-testid="subscription-history-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Link
            to="/$realmId/manage/billing"
            params={{ realmId }}
            search={{ page: 0, pageSize: 20, status: 'all' }}
          >
            <Button variant="ghost" size="icon" data-testid="back-button">
              <ArrowLeft className="h-5 w-5" />
              <span className="sr-only">Back</span>
            </Button>
          </Link>
          <div>
            <h1 className="text-3xl font-bold">Subscription History</h1>
            <p className="text-muted-foreground">View all subscription changes across the realm</p>
          </div>
        </div>
      </div>

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
          <CardTitle className="flex items-center gap-2">
            <Info className="h-5 w-5" />
            History Events
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <SubscriptionHistoryList
            events={historyData?.events || []}
            loading={isLoading}
            pagination={historyData?.pagination}
            onPageChange={handlePageChange}
            onSortChange={handleSortChange}
            onEventClick={handleEventClick}
          />
        </CardContent>
      </Card>
    </div>
  )
}
