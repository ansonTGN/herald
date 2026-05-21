import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const InvoiceFormPage = lazy(() =>
  import('@/components/billing/invoices/invoice-form-page').then((m) => ({
    default: m.InvoiceFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/billing/invoices/new')({
  component: NewInvoicePage,
})

function NewInvoicePage() {
  const { realmId } = Route.useParams()

  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-12" data-testid="invoice-form-loading">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <InvoiceFormPage mode="create" realmId={realmId} />
    </Suspense>
  )
}
