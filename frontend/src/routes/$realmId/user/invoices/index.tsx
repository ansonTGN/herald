import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { InvoiceUserPage } from '@/components/billing/invoices/invoice-user-page'

export const Route = createFileRoute('/$realmId/user/invoices/')({
  component: UserInvoicesIndexRoute,
})

function UserInvoicesIndexRoute() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()

  const handleApplyInvoice = () => {
    navigate({ to: '/$realmId/user/invoices/new', params: { realmId } })
  }

  return <InvoiceUserPage realmId={realmId} onApplyInvoice={handleApplyInvoice} />
}
