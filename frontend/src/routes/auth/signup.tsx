import { createFileRoute } from '@tanstack/react-router'
import { SignupPage } from '@/routes/$realmId/auth/signup'

export const Route = createFileRoute('/auth/signup')({
  component: SignupPage,
})
