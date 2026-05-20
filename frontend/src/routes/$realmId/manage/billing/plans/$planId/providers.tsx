import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const PlanProvidersPage = lazy(() =>
  import('@/components/billing/plan-providers-page').then((m) => ({
    default: m.PlanProvidersPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/billing/plans/$planId/providers')({
  component: PlanProvidersRoute,
})

function PlanProvidersRoute() {
  const { realmId, planId } = Route.useParams()

  return (
    <div className="container max-w-5xl mx-auto py-6 px-6">
      <Suspense
        fallback={
          <div
            className="flex items-center justify-center py-12"
            data-testid="plan-providers-loading"
          >
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <PlanProvidersPage realmId={realmId} planId={planId} />
      </Suspense>
    </div>
  )
}
