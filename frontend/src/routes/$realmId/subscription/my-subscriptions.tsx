import { createFileRoute } from '@tanstack/react-router'
import { MySubscriptionsPage } from '@/components/billing/my-subscriptions-page'
import { requireFeature } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/subscription/my-subscriptions')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.user.subscriptionVisible, {
      to: '/$realmId/user/profile',
      params: { realmId: params.realmId },
    }),
  component: MySubscriptionsRoute,
})

function MySubscriptionsRoute() {
  const { realmId } = Route.useParams()

  return <MySubscriptionsPage realmId={realmId} />
}
