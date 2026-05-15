import { createFileRoute } from '@tanstack/react-router'
import { Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/$realmId/device')({
  component: DeviceVerificationPage,
})

function DeviceVerificationPage() {
  return <Outlet />
}
