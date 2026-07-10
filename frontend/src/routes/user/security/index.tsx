import { createFileRoute } from '@tanstack/react-router'
import { ProfileSecurity } from '@/routes/$realmId/user/security/'

export const Route = createFileRoute('/user/security/')({
  component: ProfileSecurity,
})
