import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/$realmId/user/security')({
  component: () => <Outlet />,
})
