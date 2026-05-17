import { createFileRoute } from '@tanstack/react-router'
import { PaymentProvidersPage } from '@/components/billing/payment-providers-page'

export const Route = createFileRoute('/$realmId/manage/billing/payment-providers/')({
  component: PaymentProvidersRoute,
})

function PaymentProvidersRoute() {
  const { realmId } = Route.useParams()
  return <PaymentProvidersPage realmId={realmId} />
}
