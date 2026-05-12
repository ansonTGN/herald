import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/$realmId')({
  component: function RealmRoute() {
    return <Outlet />
  },
})
