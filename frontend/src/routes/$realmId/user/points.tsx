import { createFileRoute } from '@tanstack/react-router'
import { UserPointsPage } from '@/components/points/UserPointsPage'
import { useUser } from '@/stores/auth-store'
import { requireFeature } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/user/points')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.user.pointsVisible, {
      to: '/$realmId/user/profile',
      params: { realmId: params.realmId },
    }),
  component: UserPointsWrapper,
})

function UserPointsWrapper() {
  const { realmId } = Route.useParams()
  const user = useUser()
  // Get userId from auth store since this is user's own points page
  const userId = user?.id || ''
  return <UserPointsPage realmId={realmId} userId={userId} />
}
