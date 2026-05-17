import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/$realmId/manage/billing/payment-providers')({
  component: () => <Outlet />,
})
