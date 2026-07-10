import { createFileRoute } from '@tanstack/react-router'
import { DeviceVerificationView } from '@/components/device/device-verification-view'
import { resolvedRealmFromPath } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/device/')({
  component: DeviceVerificationIndexPage,
})

export function DeviceVerificationIndexPage() {
  const { realmId } = resolvedRealmFromPath(window.location.pathname)
  return <DeviceVerificationView realmId={realmId} />
}
