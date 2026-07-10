import { createFileRoute } from '@tanstack/react-router'
import { PaymentProvidersRoute } from '@/routes/$realmId/manage/billing/payment-providers/index'

export const Route = createFileRoute('/manage/billing/payment-providers/')({
  component: PaymentProvidersRoute,
})
