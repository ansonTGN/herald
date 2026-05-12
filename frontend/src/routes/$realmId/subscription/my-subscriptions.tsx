import { createFileRoute } from '@tanstack/react-router'
import { MySubscriptionsPage } from '@/components/billing/my-subscriptions-page'

export const Route = createFileRoute('/$realmId/subscription/my-subscriptions')({
  component: MySubscriptionsRoute,
})

function MySubscriptionsRoute() {
  const { realmId } = Route.useParams()

  return <MySubscriptionsPage realmId={realmId} />
}
