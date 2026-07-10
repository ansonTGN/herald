import { createFileRoute } from '@tanstack/react-router'
import { EditInvoicePage } from '@/routes/$realmId/manage/billing/invoices/$invoiceId.edit'

export const Route = createFileRoute('/manage/billing/invoices/$invoiceId/edit')({
  component: EditInvoicePage,
})
