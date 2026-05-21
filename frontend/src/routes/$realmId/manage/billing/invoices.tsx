import { createFileRoute, Outlet } from '@tanstack/react-router'
import { requireFeature } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/manage/billing/invoices')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.admin.invoicesVisible, {
      to: '/$realmId/manage/billing',
      params: { realmId: params.realmId },
      search: { status: 'all' },
    }),
  component: () => <Outlet />,
})
