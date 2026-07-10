import { createFileRoute } from '@tanstack/react-router'
import { InvoiceAdminRoute } from '@/routes/$realmId/manage/billing/invoices/index'

export const Route = createFileRoute('/manage/billing/invoices/')({
  component: InvoiceAdminRoute,
})
