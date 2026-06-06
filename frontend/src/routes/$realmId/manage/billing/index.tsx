import { createFileRoute } from '@tanstack/react-router'
import { requireFeature } from '@/data/query-options'
import { BillingPage } from '@/components/billing/billing-page'

export const Route = createFileRoute('/$realmId/manage/billing/')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.admin.billingVisible, {
      to: '/$realmId/manage',
      params: { realmId: params.realmId },
    }),
  component: BillingRoute,
})

function BillingRoute() {
  const { realmId } = Route.useParams()

  return <BillingPage realmId={realmId} />
}
