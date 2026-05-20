import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const PointsPackageProvidersPage = lazy(() =>
  import('@/components/points-packages/points-package-providers-page').then((m) => ({
    default: m.PointsPackageProvidersPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/points-packages/$packageId/providers')({
  component: PointsPackageProvidersRoute,
})

function PointsPackageProvidersRoute() {
  const { realmId, packageId } = Route.useParams()

  return (
    <Suspense
      fallback={
        <div
          className="flex items-center justify-center py-12"
          data-testid="points-package-providers-loading"
        >
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <PointsPackageProvidersPage realmId={realmId} packageId={packageId} />
    </Suspense>
  )
}
