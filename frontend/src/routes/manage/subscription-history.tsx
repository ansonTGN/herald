import { createFileRoute } from '@tanstack/react-router'
import { SubscriptionHistoryRoute } from '@/routes/$realmId/manage/subscription-history'

export const Route = createFileRoute('/manage/subscription-history')({
  component: SubscriptionHistoryRoute,
})
