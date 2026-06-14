import { createFileRoute, redirect } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { z } from 'zod'

// Apply is now only reachable from a history row with a pre-filled resource
// reference (P1-3). Exactly one of paymentAttemptId/subscriptionId is required;
// if neither is present we redirect to the Points page (which hosts the
// Purchase History tab) rather than render a form with nothing to invoice.
const invoiceApplySearchSchema = z
  .object({
    paymentAttemptId: z.string().uuid().optional(),
    subscriptionId: z.string().uuid().optional(),
    returnTo: z.string().optional(),
  })
  .refine((search) => !search.paymentAttemptId !== !search.subscriptionId, {
    message: 'Exactly one invoice reference is required',
  })

const ApplyInvoiceFormPage = lazy(() =>
  import('@/components/billing/invoices/apply-invoice-form-page').then((m) => ({
    default: m.ApplyInvoiceFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/user/invoices/new')({
  component: ApplyInvoicePageRoute,
  validateSearch: (search) => invoiceApplySearchSchema.parse(search),
  beforeLoad: ({ params, search }) => {
    const parsed = invoiceApplySearchSchema.safeParse(search)
    if (!parsed.success) {
      throw redirect({
        to: '/$realmId/user/points',
        params: { realmId: params.realmId },
      })
    }
  },
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
