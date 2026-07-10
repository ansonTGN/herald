import { createFileRoute } from '@tanstack/react-router'
import { VerifyEmailPage } from '@/routes/$realmId/auth/verify-email'

export const Route = createFileRoute('/auth/verify-email')({
  component: VerifyEmailPage,
})
