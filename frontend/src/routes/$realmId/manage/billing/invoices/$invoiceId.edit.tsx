import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { invoiceDetailQueryOptions } from '@/data/invoice-query-options'
import { useLastPathSegment, useResolvedRealmId } from '@/lib/realm-routing'

const InvoiceFormPage = lazy(() =>
  import('@/components/billing/invoices/invoice-form-page').then((m) => ({
    default: m.InvoiceFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/billing/invoices/$invoiceId/edit')({
  component: EditInvoicePage,
})

export function EditInvoicePage() {
  const realmId = useResolvedRealmId()
  const invoiceId = useLastPathSegment(1)
  const { data: invoice } = useSuspenseQuery(invoiceDetailQueryOptions(realmId, invoiceId))

  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-12" data-testid="invoice-form-loading">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <InvoiceFormPage mode="edit" realmId={realmId} invoice={invoice} />
    </Suspense>
  )
}
