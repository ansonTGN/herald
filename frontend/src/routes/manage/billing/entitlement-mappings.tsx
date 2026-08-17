import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/manage/billing/entitlement-mappings')({
  component: () => <Outlet />,
})
