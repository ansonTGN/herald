import { useState, useMemo } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import {
  subscriptionHistoryQueryOptions,
  clientAppsQueryOptions,
  userSubscriptionsQueryOptions,
} from '@/data/query-options'
import { SubscriptionSelector } from '@/components/billing/subscription-selector'
import { UserSubscriptionTimeline } from '@/components/billing/user-subscription-timeline'
import type { ClientAppItem, SubscriptionDetailResponse } from '@/lib/api-generated'
import { getSubscriptionForClientApp } from '@/lib/api-generated'
import { PageHeader } from '@/components/shared'

type SubscriptionWithClientApp = {
  clientApp: ClientAppItem
  subscription: SubscriptionDetailResponse
}

export const Route = createFileRoute('/$realmId/user/subscription-history')({
  component: SubscriptionHistoryRoute,
})

function SubscriptionHistoryRoute() {
  const { realmId } = Route.useParams()
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

  return (
    <div className="space-y-6" data-testid="subscription-history-page">
      <PageHeader
        title="Subscription History"
        description="View your subscription changes and history"
      />

      {/* Subscription Selector */}
      <div>
        <h2 className="text-lg font-semibold mb-4">Select Subscription</h2>
        {loadingApps || loadingSubscriptions ? (
          <div className="text-center py-8 text-muted-foreground">Loading subscriptions...</div>
        ) : (
          <SubscriptionSelector
            subscriptions={subscriptionList ?? []}
            selectedId={selectedSubscriptionId}
            onSelect={setManualSelectionId}
          />
        )}
      </div>

      {/* History Timeline */}
      {selectedSubscriptionId && (
        <div>
          <h2 className="text-lg font-semibold mb-4">History Timeline</h2>
          <UserSubscriptionTimeline events={historyData?.events ?? []} loading={loadingHistory} />
        </div>
      )}
    </div>
  )
}
