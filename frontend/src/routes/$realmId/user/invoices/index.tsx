import { createFileRoute } from '@tanstack/react-router'
import { InvoiceUserPage } from '@/components/billing/invoices/invoice-user-page'

export const Route = createFileRoute('/$realmId/user/invoices/')({
  component: UserInvoicesIndexRoute,
})

function UserInvoicesIndexRoute() {
  const { realmId } = Route.useParams()
  return <InvoiceUserPage realmId={realmId} />
}
