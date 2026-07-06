import { useMemo } from 'react'
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { PageHeader } from '@/components/shared/page-header'
import { formatDate } from '@/lib/date-utils'
import { clientAppsQueryOptions, userSubscriptionsQueryOptions } from '@/data/query-options'
import {
  getSubscriptionForClientApp,
  type ClientAppItem,
  type SubscriptionDetailResponse,
} from '@/lib/api-generated'
import { formatProviderName } from '@/components/billing/format-provider-name'
import { m } from '@/paraglide/messages'

interface MySubscriptionsPageProps {
  realmId: string
}

type SubscriptionWithClientApp = {
  clientApp: ClientAppItem
  subscription: SubscriptionDetailResponse
}

export function MySubscriptionsPage({ realmId }: MySubscriptionsPageProps) {
  const { data: clientAppsResponse, isLoading: isLoadingApps } = useQuery(
    clientAppsQueryOptions(realmId, { page: 0, pageSize: 100 })
  )

  const clientApps = useMemo(() => clientAppsResponse?.items ?? [], [clientAppsResponse?.items])

  const clientAppIds = useMemo(
    () =>
      clientApps
        .map((app) => app.id)
        .sort()
        .join(','),
    [clientApps]
  )

  // Query for subscriptions
  const { data: subscriptions = [], isLoading: isLoadingSubscriptions } = useQuery({
    ...userSubscriptionsQueryOptions<SubscriptionWithClientApp[]>(
      realmId,
      clientAppIds,
      async () => {
        if (clientApps.length === 0) {
          return []
        }

        const results = await Promise.all(
          clientApps.map(async (clientApp) => {
            try {
              const response = await getSubscriptionForClientApp({
                path: { realmId, clientAppId: clientApp.id },
              })

              if (response.error || !response.data) {
                return null
              }

              return {
                clientApp,
                subscription: response.data,
              }
            } catch {
              return null
            }
          })
        )

        return results.filter((result): result is SubscriptionWithClientApp => result !== null)
      }
    ),
    enabled: clientApps.length > 0,
  })

  const isLoading = isLoadingApps || isLoadingSubscriptions

  const getStatusBadge = (status: string) => {
    switch (status.toLowerCase()) {
      case 'active':
        return (
          <Badge className="bg-green-100 text-green-800">
            {m['billing.subscription_status_label_active']()}
          </Badge>
        )
      case 'past_due':
        return (
          <Badge variant="destructive">{m['billing.subscription_status_label_past_due']()}</Badge>
        )
      case 'canceled':
        return (
          <Badge variant="secondary">{m['billing.subscription_status_label_canceled']()}</Badge>
        )
      case 'scheduled_cancel':
        return (
          <Badge className="bg-yellow-100 text-yellow-800">
            {m['billing.subscription_status_label_scheduled_cancel']()}
          </Badge>
        )
      default:
        return <Badge variant="outline">{status}</Badge>
    }
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="text-muted-foreground">{m['billing.my_subscriptions_loading']()}</div>
      </div>
    )
  }

  return (
    <div className="space-y-6" data-testid="my-subscriptions-page">
      <PageHeader title={m['billing.my_subscriptions_title']()} />

      {subscriptions.length === 0 ? (
        <Card className="border-dashed">
          <CardContent className="flex flex-col items-center justify-center py-12">
            <div className="text-4xl mb-4">📦</div>
            <h3 className="text-lg font-semibold mb-2">
              {m['billing.my_subscriptions_empty_title']()}
            </h3>
            <p className="text-sm text-muted-foreground text-center">
              {m['billing.my_subscriptions_empty_description']()}
            </p>
            <Button asChild className="mt-4" data-testid="my-subscriptions-browse-plans">
              <Link to="/$realmId/user/purchase-points" params={{ realmId }}>
                {m['billing.my_subscriptions_browse_plans']()}
              </Link>
            </Button>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3" data-testid="subscription-list">
          {subscriptions.map(({ clientApp, subscription }) => (
            <Card key={subscription.id}>
              <CardHeader>
                <div className="flex items-start justify-between">
                  <div>
                    <CardTitle className="text-lg">
                      {subscription.entitlementKey ?? 'Subscription'}
                    </CardTitle>
                    <CardDescription className="mt-1">{clientApp.name}</CardDescription>
                  </div>
                  {getStatusBadge(subscription.status)}
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                <div>
                  <div className="text-sm text-muted-foreground">
                    {m['billing.my_subscriptions_current_period']()}
                  </div>
                  <div className="text-sm font-medium">
                    {subscription.currentPeriodStart && subscription.currentPeriodEnd
                      ? `${formatDate(subscription.currentPeriodStart)} - ${formatDate(subscription.currentPeriodEnd)}`
                      : m['billing.my_subscriptions_not_available']()}
                  </div>
                </div>

                <div>
                  <div className="text-sm text-muted-foreground">
                    {m['billing.subscription_payment_provider']()}
                  </div>
                  <div className="text-2xl font-bold">
                    {formatProviderName(subscription.paymentProvider ?? '')}
                  </div>
                </div>

                {subscription.status.toLowerCase() === 'active' && (
                  <Button
                    asChild
                    variant="secondary"
                    size="sm"
                    className="w-full"
                    data-testid={`subscription-change-plan-${subscription.id}`}
                  >
                    <Link to="/$realmId/user/purchase-points" params={{ realmId }}>
                      {m['billing.my_subscriptions_change_plan']()}
                    </Link>
                  </Button>
                )}

                <Button asChild variant="outline" size="sm" className="w-full">
                  <Link
                    to="/$realmId/subscription/$subscriptionId/history"
                    params={{ realmId, subscriptionId: subscription.id }}
                  >
                    {m['billing.my_subscriptions_view_history']()}
                  </Link>
                </Button>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  )
}
