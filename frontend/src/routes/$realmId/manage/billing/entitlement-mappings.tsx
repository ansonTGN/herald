import { createFileRoute } from '@tanstack/react-router'
import { z } from 'zod'
import { requireFeature } from '@/data/query-options'
import { EntitlementMappingsPage } from '@/components/billing/entitlement-mappings-page'

const entitlementMappingsSearchSchema = z.object({
  page: z.number().int().min(0).optional(),
  pageSize: z.number().int().min(1).max(100).optional(),
  provider: z.string().optional(),
})

export const Route = createFileRoute('/$realmId/manage/billing/entitlement-mappings')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.admin.entitlementMappingsVisible, {
      to: '/$realmId/manage',
      params: { realmId: params.realmId },
    }),
  validateSearch: entitlementMappingsSearchSchema,
  component: EntitlementMappingsRoute,
})

function EntitlementMappingsRoute() {
  const { realmId } = Route.useParams()
  const search = Route.useSearch()

  return <EntitlementMappingsPage realmId={realmId} search={search} />
}
