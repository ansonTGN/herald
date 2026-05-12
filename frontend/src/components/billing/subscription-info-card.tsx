import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { subscriptionQueryOptions } from '@/data/query-options'
import { getStatusBadgeVariant, type SubscriptionStatus } from '@/types/billing'
import { formatDate } from '@/lib/date-utils'

interface SubscriptionInfoCardProps {
  realmId: string
  clientAppId: string
  clientAppName: string
}

export function SubscriptionInfoCard({
  realmId,
  clientAppId,
  clientAppName,
}: SubscriptionInfoCardProps) {
  const {
    data: subscription,
    isLoading,
    error,
  } = useQuery(subscriptionQueryOptions(realmId, clientAppId))

  if (isLoading) {
    return (
      <Card data-testid="subscription-info-card-loading">
        <CardContent className="py-6">
          <div className="animate-pulse space-y-3">
            <div className="h-4 bg-gray-200 rounded w-1/4" />
            <div className="h-3 bg-gray-200 rounded w-1/3" />
            <div className="h-3 bg-gray-200 rounded w-1/2" />
          </div>
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Card data-testid="subscription-info-card-error">
        <CardContent className="py-6 text-center text-destructive">
          Failed to load subscription information
        </CardContent>
      </Card>
    )
  }

  if (!subscription) {
    return (
      <Card data-testid="subscription-info-card-empty">
        <CardHeader>
          <CardTitle className="text-base">{clientAppName}</CardTitle>
        </CardHeader>
        <CardContent className="py-4">
          <p className="text-sm text-muted-foreground">No active subscription</p>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card data-testid={`subscription-info-card-${clientAppId}`}>
      <CardHeader>
        <CardTitle className="text-base">{clientAppName}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">Status</span>
          <Badge
            variant={getStatusBadgeVariant(subscription.status as SubscriptionStatus)}
            data-testid={`subscription-status-${clientAppId}`}
          >
            {subscription.status}
          </Badge>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">Plan</span>
          <span className="text-sm" data-testid={`subscription-plan-${clientAppId}`}>
            {subscription.plan?.title || 'None'}
          </span>
        </div>

        {subscription.currentPeriodStart && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">Period Start</span>
            <span className="text-sm">{formatDate(subscription.currentPeriodStart)}</span>
          </div>
        )}

        {subscription.currentPeriodEnd && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">Period End</span>
            <span className="text-sm">{formatDate(subscription.currentPeriodEnd)}</span>
          </div>
        )}

        {subscription.cancelAt && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">Cancel At</span>
            <span className="text-sm text-muted-foreground">
              {formatDate(subscription.cancelAt)}
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
