import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/$realmId/manage/billing')({
  component: BillingRoute,
})

function BillingRoute() {
  return <Outlet />
}
