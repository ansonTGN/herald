import { createFileRoute } from '@tanstack/react-router'
import { DeviceVerificationView } from '@/components/device/device-verification-view'
import { resolvedRealmFromPath } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/device/$userCode')({
  component: DeviceVerificationWithCodePage,
})

export function DeviceVerificationWithCodePage() {
  const pathname = window.location.pathname
  const { realmId } = resolvedRealmFromPath(pathname)
  const userCode = pathname.split('/').filter(Boolean).at(-1) ?? ''
  return <DeviceVerificationView realmId={realmId} initialCode={userCode} />
}
