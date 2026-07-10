import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/manage/api-keys')({
  component: () => <Outlet />,
})
