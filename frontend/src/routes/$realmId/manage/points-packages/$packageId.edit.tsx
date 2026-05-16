import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { pointsPackageQueryOptions } from '@/data/query-options'

const PointsPackageFormPage = lazy(() =>
  import('@/components/points-packages/points-package-form-page').then((m) => ({
    default: m.PointsPackageFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/points-packages/$packageId/edit')({
  component: EditPointsPackagePage,
})

function EditPointsPackagePage() {
  const { realmId, packageId } = Route.useParams()
  const { data: pkg } = useSuspenseQuery(pointsPackageQueryOptions(realmId, packageId))

  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-12" data-testid="points-package-form-loading">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <PointsPackageFormPage mode="edit" realmId={realmId} pkg={pkg} />
    </Suspense>
  )
}
