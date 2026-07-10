import { createFileRoute } from '@tanstack/react-router'
import { SubscriptionsRoute } from '@/routes/$realmId/manage/billing/subscriptions'

export const Route = createFileRoute('/manage/billing/subscriptions')({
  component: SubscriptionsRoute,
})
