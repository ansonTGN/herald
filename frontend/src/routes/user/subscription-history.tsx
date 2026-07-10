import { createFileRoute } from '@tanstack/react-router'
import { PurchaseRecordsRoute } from '@/routes/$realmId/user/subscription-history'

export const Route = createFileRoute('/user/subscription-history')({
  component: PurchaseRecordsRoute,
})
