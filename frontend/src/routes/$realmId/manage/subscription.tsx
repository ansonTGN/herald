import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { SubscriptionManagement } from '@/components/billing/subscription-management'
import { clientAppsQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'
import { useResolvedRealmId } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/manage/subscription')({
  component: SubscriptionRoute,
})

export function SubscriptionRoute() {
  const realmId = useResolvedRealmId()

  const { data: clientAppsResponse } = useSuspenseQuery(
    clientAppsQueryOptions(realmId, { page: 0, pageSize: 10 })
  )

  // Use the first client app's ID for subscription (admin-web-console)
  const clientAppId = clientAppsResponse.items[0]?.id ?? ''

  if (!clientAppId) {
    return (
      <div className="p-4 text-center text-gray-600" data-testid="no-client-app-message">
        {m['billing.subscription_no_client_app']()}
      </div>
    )
  }

  return (
    <div data-testid="subscription-route">
      <SubscriptionManagement realmId={realmId} clientAppId={clientAppId} />
    </div>
  )
}
