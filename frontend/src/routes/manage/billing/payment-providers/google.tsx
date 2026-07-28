import { createFileRoute } from '@tanstack/react-router'
import { GoogleConfigRoute } from '@/routes/$realmId/manage/billing/payment-providers/google'

export const Route = createFileRoute('/manage/billing/payment-providers/google')({
  component: GoogleConfigRoute,
})
