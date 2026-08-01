import { createFileRoute, Outlet } from '@tanstack/react-router'
import { requireUserFeature } from '@/data/query-options'

export const Route = createFileRoute('/$realmId/user/invoices')({
  beforeLoad: ({ context, params }) =>
    requireUserFeature(context.queryClient, (f) => f.user.invoicesVisible, {
      to: '/$realmId/user/profile',
      params: { realmId: params.realmId },
    }),
  component: () => <Outlet />,
})
