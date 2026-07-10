import { createFileRoute } from '@tanstack/react-router'
import { ProfileIndex } from '@/routes/$realmId/user/profile'

export const Route = createFileRoute('/user/profile')({
  component: ProfileIndex,
})
