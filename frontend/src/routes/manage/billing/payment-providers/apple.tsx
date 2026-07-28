import { createFileRoute } from '@tanstack/react-router'
import { AppleConfigRoute } from '@/routes/$realmId/manage/billing/payment-providers/apple'

export const Route = createFileRoute('/manage/billing/payment-providers/apple')({
  component: AppleConfigRoute,
})
