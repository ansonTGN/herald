import { createFileRoute } from '@tanstack/react-router'
import { PointsConfigsPage } from '@/components/points/configs/PointsConfigsPage'

export const Route = createFileRoute('/$realmId/manage/points/configs/')({
  component: PointsConfigsRoute,
})

function PointsConfigsRoute() {
  const { realmId } = Route.useParams()
  return <PointsConfigsPage realmId={realmId} />
}
