import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { subscriptionPlanQueryOptions } from '@/data/query-options'

const PlanFormPage = lazy(() =>
  import('@/components/billing/plan-form-page').then((m) => ({
    default: m.PlanFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/billing/plans/$planId/edit')({
  component: EditPlanPage,
})

function EditPlanPage() {
  const { realmId, planId } = Route.useParams()
  const { data: plan } = useSuspenseQuery(subscriptionPlanQueryOptions(realmId, planId))

  return (
    <div className="container max-w-3xl mx-auto py-6 px-6">
      <Suspense
        fallback={
          <div className="flex items-center justify-center py-12" data-testid="plan-form-loading">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <PlanFormPage mode="edit" realmId={realmId} plan={plan} />
      </Suspense>
    </div>
  )
}
