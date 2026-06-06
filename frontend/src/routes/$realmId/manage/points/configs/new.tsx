import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const PointsPlanConfigFormPage = lazy(() =>
  import('@/components/points/configs/PointsPlanConfigFormPage').then((m) => ({
    default: m.PointsPlanConfigFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/points/configs/new')({
  component: NewPointsPlanConfigPage,
})

function NewPointsPlanConfigPage() {
  const { realmId } = Route.useParams()

  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-12" data-testid="points-rule-loading">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <PointsPlanConfigFormPage mode="create" realmId={realmId} plans={[]} />
    </Suspense>
  )
}
