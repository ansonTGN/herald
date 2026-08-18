import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { ChevronDown, ChevronUp } from 'lucide-react'
import { subscriptionQueryOptions } from '@/data/query-options'
import {
  getStatusBadgeVariant,
  getSubscriptionStatusLabels,
  type SubscriptionStatus,
} from '@/types/billing'
import { formatDate } from '@/lib/date-utils'
import { m } from '@/paraglide/messages'

interface SubscriptionInfoCardProps {
  realmId: string
  clientAppId: string
  clientAppName: string
}

function ProviderMetadataSection({ metadata }: { metadata: unknown }) {
  const [expanded, setExpanded] = useState(false)

  if (!metadata || (typeof metadata === 'object' && Object.keys(metadata as object).length === 0)) {
    return (
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">{m['billing.subscription_provider_metadata']()}</span>
        <span className="text-sm text-muted-foreground">
          {m['billing.subscription_no_provider_metadata']()}
        </span>
      </div>
    )
  }

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium">{m['billing.subscription_provider_metadata']()}</span>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setExpanded(!expanded)}
          className="h-7 px-2 text-xs"
        >
          {expanded ? (
            <>
              <ChevronUp className="h-3 w-3 mr-1" />
              {m['billing.subscription_hide_details']()}
            </>
          ) : (
            <>
              <ChevronDown className="h-3 w-3 mr-1" />
              {m['billing.subscription_show_details']()}
            </>
          )}
        </Button>
      </div>
      {expanded && (
        <pre className="text-xs bg-muted p-2 rounded overflow-x-auto">
          {JSON.stringify(metadata, null, 2)}
        </pre>
      )}
    </div>
  )
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
            <div className="h-4 bg-muted rounded w-1/4" />
            <div className="h-3 bg-muted rounded w-1/3" />
            <div className="h-3 bg-muted rounded w-1/2" />
          </div>
        </CardContent>
      </Card>
    )
  }

  if (error) {
    return (
      <Card data-testid="subscription-info-card-error">
        <CardContent className="py-6 text-center text-destructive">
          {m['billing.subscription_failed_load']()}
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
          <p className="text-sm text-muted-foreground">{m['billing.subscription_no_active']()}</p>
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
          <span className="text-sm font-medium">{m['billing.subscription_status']()}</span>
          <Badge
            variant={getStatusBadgeVariant(subscription.status as SubscriptionStatus)}
            data-testid={`subscription-status-${clientAppId}`}
          >
            {getSubscriptionStatusLabels()[subscription.status as SubscriptionStatus]}
          </Badge>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">{m['billing.subscription_entitlement_key']()}</span>
          <span className="text-sm" data-testid={`subscription-entitlement-key-${clientAppId}`}>
            {subscription.entitlementKey || m['billing.subscription_none']()}
          </span>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm font-medium">
            {m['billing.subscription_payment_provider']()}
          </span>
          <span className="text-sm">
            {subscription.paymentProvider || m['billing.subscription_none']()}
          </span>
        </div>

        {subscription.externalPriceId && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">
              {m['billing.subscription_external_price_id']()}
            </span>
            <span className="text-sm">{subscription.externalPriceId}</span>
          </div>
        )}

        {subscription.syncedAt && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">{m['billing.subscription_synced_at']()}</span>
            <span className="text-sm">{formatDate(subscription.syncedAt)}</span>
          </div>
        )}

        <ProviderMetadataSection metadata={subscription.providerMetadata} />

        {subscription.currentPeriodStart && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">{m['billing.subscription_period_start']()}</span>
            <span className="text-sm">{formatDate(subscription.currentPeriodStart)}</span>
          </div>
        )}

        {subscription.currentPeriodEnd && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">{m['billing.subscription_period_end']()}</span>
            <span className="text-sm">{formatDate(subscription.currentPeriodEnd)}</span>
          </div>
        )}

        {subscription.cancelAt && (
          <div className="flex items-center justify-between">
            <span className="text-sm font-medium">{m['billing.subscription_cancel_at']()}</span>
            <span className="text-sm text-muted-foreground">
              {formatDate(subscription.cancelAt)}
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  )
}
