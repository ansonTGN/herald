import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/manage/billing/credit-buckets')({
  component: () => <Outlet />,
})
