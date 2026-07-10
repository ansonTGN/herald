import { createFileRoute } from '@tanstack/react-router'
import { DeviceVerificationWithCodePage } from '@/routes/$realmId/device.$userCode'

export const Route = createFileRoute('/device/$userCode')({
  component: DeviceVerificationWithCodePage,
})
