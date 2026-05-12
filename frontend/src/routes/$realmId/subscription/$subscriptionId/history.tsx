import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, Info } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { SubscriptionTimeline } from '@/components/billing/subscription-timeline'
import { queryKeys, subscriptionHistoryQueryOptions } from '@/data/query-options'
import { toast } from 'sonner'

export const Route = createFileRoute('/$realmId/subscription/$subscriptionId/history')({
  component: SubscriptionDetailHistoryRoute,
})

function SubscriptionDetailHistoryRoute() {
  const { realmId, subscriptionId } = Route.useParams()

  // Query subscription history
  const {
    data: historyData,
    isLoading: historyLoading,
    error: historyError,
  } = useQuery(subscriptionHistoryQueryOptions(realmId, subscriptionId))

  // Query subscription details
  const { data: subscription, isLoading: subscriptionLoading } = useQuery({
    queryKey: queryKeys.subscriptionDetails(realmId, subscriptionId),
    queryFn: async () => {
      // For now, we'll use the history data to get subscription info
      // In the future, this could call a dedicated subscription detail endpoint
      return historyData?.events?.[0]?.newState
    },
    enabled: !!historyData?.events?.length,
  })

  if (historyError) {
    return (
      <div className="container mx-auto px-4 py-8">
        <Card className="border-destructive">
          <CardContent className="p-6">
            <p className="text-destructive">
              Failed to load subscription history:{' '}
              {historyError instanceof Error ? historyError.message : 'Unknown error'}
            </p>
            <Button variant="outline" className="mt-4" onClick={() => window.location.reload()}>
              Retry
            </Button>
          </CardContent>
        </Card>
      </div>
    )
  }

  return (
    <div
      className="container mx-auto space-y-6 px-4 py-8"
      data-testid="subscription-detail-history-page"
    >
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button
            variant="ghost"
            size="icon"
            data-testid="back-button"
            onClick={() => window.history.back()}
          >
            <ArrowLeft className="h-5 w-5" />
            <span className="sr-only">Back</span>
          </Button>
          <div>
            <h1 className="text-3xl font-bold">Subscription History</h1>
            <p className="text-muted-foreground">
              View the complete change timeline for this subscription
            </p>
          </div>
        </div>
      </div>

      {/* Subscription Info */}
      {!subscriptionLoading && subscription && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Info className="h-5 w-5" />
              Current Subscription Status
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              <div>
                <p className="text-sm font-medium text-muted-foreground">Status</p>
                <p className="text-lg font-semibold">{subscription.status}</p>
              </div>
              {subscription.planId && (
                <div>
                  <p className="text-sm font-medium text-muted-foreground">Plan</p>
                  <p className="text-lg font-semibold">{subscription.planId}</p>
                </div>
              )}
              <div>
                <p className="text-sm font-medium text-muted-foreground">Billing Period</p>
                <p className="text-lg font-semibold">{subscription.billingPeriod}</p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Timeline */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Info className="h-5 w-5" />
            Change Timeline
          </CardTitle>
        </CardHeader>
        <CardContent>
          <SubscriptionTimeline
            events={historyData?.events || []}
            loading={historyLoading}
            onEventClick={(event) => {
              toast.info(`Event: ${event.eventType}`, {
                description: `Timestamp: ${new Date(event.timestamp).toLocaleString()}`,
              })
            }}
          />
        </CardContent>
      </Card>
    </div>
  )
}
