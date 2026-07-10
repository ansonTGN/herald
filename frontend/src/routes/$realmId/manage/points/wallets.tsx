import { createFileRoute } from '@tanstack/react-router'
import { PointsWalletsPage } from '@/components/points/wallets/PointsWalletsPage'
import { useResolvedRealmId } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/manage/points/wallets')({
  component: PointsWalletsRoute,
})

export function PointsWalletsRoute() {
  const realmId = useResolvedRealmId()
  return <PointsWalletsPage realmId={realmId} />
}
