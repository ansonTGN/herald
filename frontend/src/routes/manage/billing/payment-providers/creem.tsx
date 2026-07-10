import { createFileRoute } from '@tanstack/react-router'
import { CreemConfigRoute } from '@/routes/$realmId/manage/billing/payment-providers/creem'

export const Route = createFileRoute('/manage/billing/payment-providers/creem')({
  component: CreemConfigRoute,
})
