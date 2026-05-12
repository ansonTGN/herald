import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const PlanFormPage = lazy(() =>
  import('@/components/billing/plan-form-page').then((m) => ({
    default: m.PlanFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/billing/plans/new')({
  component: NewPlanPage,
})

function NewPlanPage() {
  const { realmId } = Route.useParams()

  return (
    <div className="container max-w-3xl mx-auto py-6 px-6">
      <Suspense
        fallback={
          <div className="flex items-center justify-center py-12" data-testid="plan-form-loading">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <PlanFormPage mode="create" realmId={realmId} />
      </Suspense>
    </div>
  )
}
