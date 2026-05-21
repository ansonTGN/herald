import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { invoiceDetailQueryOptions } from '@/data/invoice-query-options'

const InvoiceFormPage = lazy(() =>
  import('@/components/billing/invoices/invoice-form-page').then((m) => ({
    default: m.InvoiceFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/billing/invoices/$invoiceId/edit')({
  component: EditInvoicePage,
})

function EditInvoicePage() {
  const { realmId, invoiceId } = Route.useParams()
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
