import { createFileRoute } from '@tanstack/react-router'
import { DeviceVerificationIndexPage } from '@/routes/$realmId/device.index'

export const Route = createFileRoute('/device/')({
  component: DeviceVerificationIndexPage,
})
