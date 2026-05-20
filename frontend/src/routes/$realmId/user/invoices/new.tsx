import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { z } from 'zod'

const invoiceApplySearchSchema = z
  .object({
    paymentAttemptId: z.string().uuid().optional(),
    subscriptionId: z.string().uuid().optional(),
    returnTo: z.string().optional(),
  })
  .refine((search) => !(search.paymentAttemptId && search.subscriptionId), {
    message: 'Only one invoice reference is allowed',
  })

const ApplyInvoiceFormPage = lazy(() =>
  import('@/components/billing/invoices/apply-invoice-form-page').then((m) => ({
    default: m.ApplyInvoiceFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/user/invoices/new')({
  component: ApplyInvoicePageRoute,
  validateSearch: (search) => invoiceApplySearchSchema.parse(search),
})

function ApplyInvoicePageRoute() {
  const { realmId } = Route.useParams()
  const search = Route.useSearch()
  const prefilledReference = search.paymentAttemptId
    ? ({ type: 'paymentAttempt', id: search.paymentAttemptId } as const)
    : search.subscriptionId
      ? ({ type: 'subscription', id: search.subscriptionId } as const)
      : undefined

  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-12" data-testid="apply-form-loading">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <ApplyInvoiceFormPage
        realmId={realmId}
        prefilledReference={prefilledReference}
        returnTo={search.returnTo}
      />
    </Suspense>
  )
}
