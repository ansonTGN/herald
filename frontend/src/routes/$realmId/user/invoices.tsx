import { createFileRoute, Outlet } from '@tanstack/react-router'
import { requireFeature } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/user/invoices')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.user.invoicesVisible, {
      to: '/$realmId/user/profile',
      params: { realmId: params.realmId },
    }),
  component: () => <Outlet />,
})
