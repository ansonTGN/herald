import { createFileRoute } from '@tanstack/react-router'
import { DeviceVerificationView } from '@/components/device/device-verification-view'

export const Route = createFileRoute('/$realmId/device/$userCode')({
  component: DeviceVerificationWithCodePage,
})

function DeviceVerificationWithCodePage() {
  const { realmId, userCode } = Route.useParams()
  return <DeviceVerificationView realmId={realmId} initialCode={userCode} />
}
