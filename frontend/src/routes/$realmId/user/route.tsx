import { createFileRoute } from '@tanstack/react-router'
import { ProfileLayout } from '@/components/layouts/profile-layout'

export const Route = createFileRoute('/$realmId/user')({
  // NOTE: Authentication is already checked by __root.tsx
  // No need to duplicate the check here
  component: ProfileLayout,
})
