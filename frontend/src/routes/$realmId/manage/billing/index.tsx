/* eslint-disable react-refresh/only-export-components */
import { createFileRoute } from '@tanstack/react-router'
import { z } from 'zod'
import { BillingPage } from '@/components/billing/billing-page'
import { requireFeature } from '@/data/query-options'

export const billingSearchSchema = z.object({
  page: z.number().int().min(0).optional(),
  pageSize: z.number().int().min(1).max(100).optional(),
  status: z.enum(['all', 'active', 'disabled']).default('all'),
})
export type BillingSearchSchema = z.infer<typeof billingSearchSchema>

export const Route = createFileRoute('/$realmId/manage/billing/')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.admin.plansVisible, {
      to: '/$realmId/manage/products',
      params: { realmId: params.realmId },
    }),
  component: BillingRoute,
  validateSearch: (search) => billingSearchSchema.parse(search),
})

function BillingRoute() {
  const { realmId } = Route.useParams()
  const search = billingSearchSchema.parse(Route.useSearch())

  return <BillingPage realmId={realmId} search={search} />
}
