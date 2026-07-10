import { createFileRoute } from '@tanstack/react-router'
import { transactionBucketSearchSchema } from '@/lib/schemas/points-forms'
import { UserPointsWrapper } from '@/routes/$realmId/user/points'

export const Route = createFileRoute('/user/points')({
  validateSearch: transactionBucketSearchSchema,
  component: UserPointsWrapper,
})
