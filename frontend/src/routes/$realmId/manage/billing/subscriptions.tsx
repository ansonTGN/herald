import { createFileRoute } from '@tanstack/react-router'
import { z } from 'zod'
import { requireFeature } from '@/data/query-options'
import { AdminSubscriptionListPage } from '@/components/billing/admin-subscription-list-page'

const subscriptionsSearchSchema = z.object({
  page: z.number().int().min(0).optional(),
  pageSize: z.number().int().min(1).max(100).optional(),
  entitlementKey: z.string().optional(),
  status: z.string().optional(),
  paymentProvider: z.string().optional(),
})

export const Route = createFileRoute('/$realmId/manage/billing/subscriptions')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.admin.entitlementMappingsVisible, {
      to: '/$realmId/manage',
      params: { realmId: params.realmId },
    }),
  validateSearch: subscriptionsSearchSchema,
  component: SubscriptionsRoute,
})

function SubscriptionsRoute() {
  const { realmId } = Route.useParams()
  const search = Route.useSearch()

  return <AdminSubscriptionListPage realmId={realmId} search={search} />
}
