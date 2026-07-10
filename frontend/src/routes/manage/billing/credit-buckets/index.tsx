import { createFileRoute } from '@tanstack/react-router'
import { CreditBucketsIndexRoute } from '@/routes/$realmId/manage/billing/credit-buckets/index'

export const Route = createFileRoute('/manage/billing/credit-buckets/')({
  component: CreditBucketsIndexRoute,
})
