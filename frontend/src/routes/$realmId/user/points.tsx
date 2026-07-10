import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { UserPointsPage } from '@/components/points/UserPointsPage'
import { useUser } from '@/stores/auth-store'
import { requireFeature } from '@/data/query-options'
import { transactionBucketSearchSchema } from '@/lib/schemas/points-forms'
import { realmPath, useCurrentSearch, useResolvedRealmContext } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/user/points')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.user.pointsVisible, {
      to: '/$realmId/user/profile',
      params: { realmId: params.realmId },
    }),
  // `bucketId` is the shareable transaction-bucket filter
  // (`?bucketId=`). Parsing + URL ↔ filter sync covered by the
  // frontend/test slot.
  validateSearch: transactionBucketSearchSchema,
  component: UserPointsWrapper,
})

export function UserPointsWrapper() {
  const realmContext = useResolvedRealmContext()
  const realmId = realmContext.realmId
  const user = useUser()
  // Get userId from auth store since this is user's own points page
  const userId = user?.id || ''
  const search = useCurrentSearch<{ bucketId?: string }>()
  const navigate = useNavigate()

  function handleBucketIdChange(bucketId: string | undefined) {
    navigate({
      to: realmPath(realmContext, '/user/points'),
      search: () => ({ bucketId }),
      replace: true,
    })
  }

  return (
    <UserPointsPage
      realmId={realmId}
      userId={userId}
      bucketId={search.bucketId}
      onBucketIdChange={handleBucketIdChange}
    />
  )
}
