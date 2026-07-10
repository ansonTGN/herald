import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/manage/client-apps')({
  component: () => <Outlet />,
})
