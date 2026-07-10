import { createFileRoute } from '@tanstack/react-router'
import { TotpSetupPageRoute } from '@/routes/$realmId/user/security/totp-setup'

export const Route = createFileRoute('/user/security/totp-setup')({
  component: TotpSetupPageRoute,
})
