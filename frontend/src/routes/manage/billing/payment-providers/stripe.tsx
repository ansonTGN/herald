import { createFileRoute } from '@tanstack/react-router'
import { StripeConfigRoute } from '@/routes/$realmId/manage/billing/payment-providers/stripe'

export const Route = createFileRoute('/manage/billing/payment-providers/stripe')({
  component: StripeConfigRoute,
})
