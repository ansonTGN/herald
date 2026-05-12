import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/$realmId/manage/client-apps')({
  component: () => <Outlet />,
})
