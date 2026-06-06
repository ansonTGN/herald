import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { pointsPlanConfigsQueryOptions } from '@/data/query-options'

const PointsPlanConfigFormPage = lazy(() =>
  import('@/components/points/configs/PointsPlanConfigFormPage').then((m) => ({
    default: m.PointsPlanConfigFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/points/configs/$configId/edit')({
  component: EditPointsPlanConfigPage,
})

function EditPointsPlanConfigPage() {
  const { realmId, configId } = Route.useParams()
  const { data: configs } = useSuspenseQuery(pointsPlanConfigsQueryOptions(realmId))
  const config = configs.find((item) => item.configId === configId)

  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center py-12" data-testid="points-rule-loading">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <PointsPlanConfigFormPage mode="edit" realmId={realmId} config={config} plans={[]} />
    </Suspense>
  )
}
