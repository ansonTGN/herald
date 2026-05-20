import { createFileRoute, Outlet } from '@tanstack/react-router'
import { requireFeature } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/manage/billing/plans')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.admin.plansVisible, {
      to: '/$realmId/manage/products',
      params: { realmId: params.realmId },
    }),
  component: () => <Outlet />,
})
