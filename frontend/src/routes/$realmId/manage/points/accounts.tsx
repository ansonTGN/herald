import { createFileRoute } from '@tanstack/react-router'
import { PointsAccountsPage } from '@/components/points/accounts/PointsAccountsPage'

export const Route = createFileRoute('/$realmId/manage/points/accounts')({
  component: PointsAccountsRoute,
})

function PointsAccountsRoute() {
  const { realmId } = Route.useParams()
  return <PointsAccountsPage realmId={realmId} />
}
