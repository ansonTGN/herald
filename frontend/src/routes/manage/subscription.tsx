import { createFileRoute } from '@tanstack/react-router'
import { SubscriptionRoute } from '@/routes/$realmId/manage/subscription'

export const Route = createFileRoute('/manage/subscription')({
  component: SubscriptionRoute,
})
