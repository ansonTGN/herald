import { createFileRoute } from '@tanstack/react-router'
import { MySubscriptionsPage } from '@/components/billing/my-subscriptions-page'
import { requireUserFeature } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/subscription/my-subscriptions')({
  beforeLoad: ({ context, params }) =>
    requireUserFeature(context.queryClient, (f) => f.user.subscriptionVisible, {
      to: '/$realmId/user/profile',
      params: { realmId: params.realmId },
    }),
  component: MySubscriptionsRoute,
})

function MySubscriptionsRoute() {
  const { realmId } = Route.useParams()

  return <MySubscriptionsPage realmId={realmId} />
}
