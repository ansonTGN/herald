import { createFileRoute } from '@tanstack/react-router'
import { NewInvoicePage } from '@/routes/$realmId/manage/billing/invoices/new'

export const Route = createFileRoute('/manage/billing/invoices/new')({
  component: NewInvoicePage,
})
