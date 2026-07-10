import { createFileRoute } from '@tanstack/react-router'
import { ProfileLayout } from '@/components/layouts/profile-layout'

export const Route = createFileRoute('/user')({
  component: ProfileLayout,
})
