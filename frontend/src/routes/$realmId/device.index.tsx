import { createFileRoute } from '@tanstack/react-router'
import { DeviceVerificationView } from '@/components/device/device-verification-view'

export const Route = createFileRoute('/$realmId/device/')({
  component: DeviceVerificationIndexPage,
})

function DeviceVerificationIndexPage() {
  const { realmId } = Route.useParams()
  return <DeviceVerificationView realmId={realmId} />
}
