import { createFileRoute } from '@tanstack/react-router'
import { InvoiceUserPage } from '@/components/billing/invoices/invoice-user-page'

export const Route = createFileRoute('/$realmId/user/invoices')({
  component: UserInvoicesRoute,
})

function UserInvoicesRoute() {
  const { realmId } = Route.useParams()

  return <InvoiceUserPage realmId={realmId} />
}
