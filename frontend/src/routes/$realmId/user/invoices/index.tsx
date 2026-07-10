import { useState } from 'react'
import { createFileRoute } from '@tanstack/react-router'
import { InvoiceDetailDialog } from '@/components/billing/invoices/invoice-detail-dialog'
import { InvoiceUserPage } from '@/components/billing/invoices/invoice-user-page'
import type { InvoiceResponse } from '@/lib/api-generated'
import { useResolvedRealmId } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/user/invoices/')({
  component: UserInvoicesIndexRoute,
})

export function UserInvoicesIndexRoute() {
  const realmId = useResolvedRealmId()
  const [detailInvoiceId, setDetailInvoiceId] = useState<string | null>(null)

  const handleViewInvoice = (invoice: InvoiceResponse) => {
    setDetailInvoiceId(invoice.id)
  }

  return (
    <>
      <InvoiceUserPage realmId={realmId} onViewInvoice={handleViewInvoice} />

      <InvoiceDetailDialog
        variant="user"
        open={!!detailInvoiceId}
        onOpenChange={(open) => {
          if (!open) setDetailInvoiceId(null)
        }}
        realmId={realmId}
        invoiceId={detailInvoiceId}
      />
    </>
  )
}
