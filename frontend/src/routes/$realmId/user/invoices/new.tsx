import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const ApplyInvoiceFormPage = lazy(() =>
  import('@/components/billing/invoices/apply-invoice-form-page').then((m) => ({
    default: m.ApplyInvoiceFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/user/invoices/new')({
  component: ApplyInvoicePageRoute,
})

function ApplyInvoicePageRoute() {
  const { realmId } = Route.useParams()

  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-12" data-testid="apply-form-loading">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <ApplyInvoiceFormPage realmId={realmId} />
    </Suspense>
  )
}
