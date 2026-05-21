import { createFileRoute } from '@tanstack/react-router'
import { PointsWalletsPage } from '@/components/points/wallets/PointsWalletsPage'

export const Route = createFileRoute('/$realmId/manage/points/wallets')({
  component: PointsWalletsRoute,
})

function PointsWalletsRoute() {
  const { realmId } = Route.useParams()
  return <PointsWalletsPage realmId={realmId} />
}
