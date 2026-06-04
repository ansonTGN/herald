import { useState, useMemo } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import {
  subscriptionHistoryQueryOptions,
  clientAppsQueryOptions,
  userSubscriptionsQueryOptions,
  requireFeature,
  featureAvailabilityQueryOptions,
} from '@/data/query-options'
import { SubscriptionSelector } from '@/components/billing/subscription-selector'
import { UserSubscriptionTimeline } from '@/components/billing/user-subscription-timeline'
import type { ClientAppItem, SubscriptionDetailResponse } from '@/lib/api-generated'
import { getSubscriptionForClientApp } from '@/lib/api-generated'
import { PageHeader } from '@/components/shared'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { getStatusBadgeVariant, type SubscriptionStatus } from '@/types/billing'
import { FileText } from 'lucide-react'
import { m } from '@/paraglide/messages'

type SubscriptionWithClientApp = {
  clientApp: ClientAppItem
  subscription: SubscriptionDetailResponse
}

export const Route = createFileRoute('/$realmId/user/subscription-history')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.user.subscriptionVisible, {
      to: '/$realmId/user/profile',
      params: { realmId: params.realmId },
    }),
  component: SubscriptionHistoryRoute,
})

function SubscriptionHistoryRoute() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()
  // undefined = auto-select first, string = manually selected this ID
  const [manualSelectionId, setManualSelectionId] = useState<string | undefined>(undefined)

  // Query client apps using existing query options
  const { data: clientAppsResponse, isLoading: loadingApps } = useQuery(
    clientAppsQueryOptions(realmId, { page: 0, pageSize: 20 })
  )

  // Memoize derived data to prevent unnecessary re-renders
  const clientApps = useMemo(() => clientAppsResponse?.items ?? [], [clientAppsResponse?.items])

  // Memoize sorted client app IDs for stable query key
  const clientAppIds = useMemo(
    () =>
      clientApps
        .map((a) => a.id)
        .sort()
        .join(','),
    [clientApps]
  )

  // Build subscription list for the selector by fetching subscriptions
  const { data: subscriptionList, isLoading: loadingSubscriptions } = useQuery({
    ...userSubscriptionsQueryOptions<SubscriptionWithClientApp[]>(
      realmId,
      clientAppIds,
      async () => {
        if (clientApps.length === 0) return []

        const subscriptions = await Promise.all(
          clientApps.map(async (app) => {
            try {
              const response = await getSubscriptionForClientApp({
                path: { realmId, clientAppId: app.id },
              })
              if (response.error) {
                return { clientApp: app, subscription: null }
              }
              return { clientApp: app, subscription: response.data }
            } catch {
              return { clientApp: app, subscription: null }
            }
          })
        )

        return subscriptions.filter(
          ({ subscription }) => subscription !== null
        ) as SubscriptionWithClientApp[]
      }
    ),
    enabled: clientApps.length > 0,
  })
  const { data: features } = useQuery(featureAvailabilityQueryOptions(realmId))
  const invoicesVisible = features?.user.invoicesVisible === true
  const handleApplyInvoice = invoicesVisible
    ? (subscriptionId: string) => {
        navigate({
          to: '/$realmId/user/invoices/new',
          params: { realmId },
          search: {
            subscriptionId,
            returnTo: `/${realmId}/user/subscription-history`,
          },
        })
      }
    : undefined

  // Derive selected subscription ID: use manual selection if user has selected, otherwise auto-select first
  const selectedSubscriptionId = useMemo(() => {
    // If user has manually selected something, use that
    if (manualSelectionId !== undefined) {
      return manualSelectionId
    }
    // Otherwise auto-select the first subscription from the list
    if (subscriptionList && subscriptionList.length > 0 && subscriptionList[0].subscription) {
      return subscriptionList[0].subscription.id
    }
    return undefined
  }, [subscriptionList, manualSelectionId])

  // Query history for selected subscription
  const { data: historyData, isLoading: loadingHistory } = useQuery({
    ...subscriptionHistoryQueryOptions(realmId, selectedSubscriptionId ?? ''),
    enabled: !!selectedSubscriptionId,
  })

  const hasMultipleSubscriptions = (subscriptionList?.length ?? 0) > 1
  const singleSubscription =
    !hasMultipleSubscriptions && subscriptionList?.length === 1 ? subscriptionList[0] : null

  return (
    <div className="space-y-6" data-testid="subscription-history-page">
      <PageHeader title={m['billing.subscription_history_page_title']()} />

      <Card>
        <CardContent className="space-y-6 pt-6">
          {loadingApps || loadingSubscriptions ? (
            <div className="text-center py-8 text-muted-foreground">
              {m['billing.my_subscriptions_loading']()}
            </div>
          ) : hasMultipleSubscriptions ? (
            <>
              <div>
                <h2 className="text-lg font-semibold mb-4">
                  {m['billing.subscription_select_title']()}
                </h2>
                <SubscriptionSelector
                  subscriptions={subscriptionList ?? []}
                  selectedId={selectedSubscriptionId}
                  onSelect={setManualSelectionId}
                  onApplyInvoice={handleApplyInvoice}
                />
              </div>

              {selectedSubscriptionId && (
                <div>
                  <h2 className="text-lg font-semibold mb-4">
                    {m['billing.subscription_history_timeline']()}
                  </h2>
                  <UserSubscriptionTimeline
                    events={historyData?.events ?? []}
                    loading={loadingHistory}
                  />
                </div>
              )}
            </>
          ) : singleSubscription ? (
            <div>
              <div className="mb-4 flex items-center justify-between gap-3">
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <span>{m['billing.subscription_showing_history_for']()}</span>
                  <span className="font-medium text-foreground">
                    {singleSubscription.clientApp.name}
                  </span>
                  {singleSubscription.subscription && (
                    <Badge
                      variant={getStatusBadgeVariant(
                        singleSubscription.subscription.status as SubscriptionStatus
                      )}
                      className="text-xs"
                    >
                      {singleSubscription.subscription.status}
                    </Badge>
                  )}
                </div>
                {handleApplyInvoice && (
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => handleApplyInvoice(singleSubscription.subscription.id)}
                    data-testid={`subscription-invoice-button-${singleSubscription.subscription.id}`}
                  >
                    <FileText className="mr-2 h-4 w-4" />
                    {m['billing.subscription_invoice_button']()}
                  </Button>
                )}
              </div>
              <UserSubscriptionTimeline
                events={historyData?.events ?? []}
                loading={loadingHistory}
              />
            </div>
          ) : (
            <div className="text-center py-8 text-muted-foreground">
              {m['billing.subscription_not_found']()}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
