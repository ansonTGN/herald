import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/$realmId/legal')({
  component: LegalLayout,
})

function LegalLayout() {
  return <Outlet />
}
