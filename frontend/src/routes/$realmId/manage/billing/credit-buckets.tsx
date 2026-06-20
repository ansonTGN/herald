import { createFileRoute, Outlet } from '@tanstack/react-router'
import { requireFeature } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/manage/billing/credit-buckets')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.admin.pointsVisible, {
      to: '/$realmId/manage',
      params: { realmId: params.realmId },
    }),
  component: () => <Outlet />,
})
