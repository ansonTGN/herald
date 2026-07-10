import { createFileRoute } from '@tanstack/react-router'
import { PaymentProvidersPage } from '@/components/billing/payment-providers-page'
import { useResolvedRealmId } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/manage/billing/payment-providers/')({
  component: PaymentProvidersRoute,
})

export function PaymentProvidersRoute() {
  const realmId = useResolvedRealmId()
  return <PaymentProvidersPage realmId={realmId} />
}
