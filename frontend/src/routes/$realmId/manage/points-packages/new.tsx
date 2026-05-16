import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const PointsPackageFormPage = lazy(() =>
  import('@/components/points-packages/points-package-form-page').then((m) => ({
    default: m.PointsPackageFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/points-packages/new')({
  component: NewPointsPackagePage,
})

function NewPointsPackagePage() {
  const { realmId } = Route.useParams()

  return (
    <Suspense
      fallback={
        <div
          className="flex items-center justify-center py-12"
          data-testid="points-package-form-loading"
        >
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <PointsPackageFormPage mode="create" realmId={realmId} />
    </Suspense>
  )
}
