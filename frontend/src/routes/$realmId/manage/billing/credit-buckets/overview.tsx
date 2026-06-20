import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { z } from 'zod'
import { CreditBucketOverviewPage } from '@/components/billing/credit-bucket/credit-bucket-overview-page'

const creditTypeKeySchema = z.enum([
  'topup',
  'subscription',
  'registration',
  'freePeriodic',
  'granted',
])

const overviewSearchSchema = z.object({
  enabledOnly: z.boolean().optional(),
  creditTypes: z.array(creditTypeKeySchema).optional(),
})

export type CreditTypeKey = z.infer<typeof creditTypeKeySchema>

export type OverviewSearch = z.infer<typeof overviewSearchSchema>

export const Route = createFileRoute('/$realmId/manage/billing/credit-buckets/overview')({
  validateSearch: overviewSearchSchema,
  component: CreditBucketOverviewRoute,
})

function CreditBucketOverviewRoute() {
  const { realmId } = Route.useParams()
  const search = Route.useSearch()
  const navigate = useNavigate()

  function handleSearchChange(next: OverviewSearch) {
    navigate({
      to: '/$realmId/manage/billing/credit-buckets/overview',
      params: { realmId },
      search: () => ({
        enabledOnly: next.enabledOnly,
        creditTypes: next.creditTypes,
      }),
      replace: true,
    })
  }

  return (
    <CreditBucketOverviewPage
      realmId={realmId}
      search={search}
      onSearchChange={handleSearchChange}
    />
  )
}
