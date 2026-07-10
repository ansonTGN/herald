import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/device')({
  component: function DeviceVerificationPage() {
    return <Outlet />
  },
})
