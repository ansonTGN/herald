import { createFileRoute } from '@tanstack/react-router'
import { PointsWalletsRoute } from '@/routes/$realmId/manage/points/wallets'

export const Route = createFileRoute('/manage/points/wallets')({
  component: PointsWalletsRoute,
})
