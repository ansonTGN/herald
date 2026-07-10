import { createFileRoute } from '@tanstack/react-router'
import { ForgotPasswordPage } from '@/routes/$realmId/auth/forgot-password'

export const Route = createFileRoute('/auth/forgot-password')({
  component: ForgotPasswordPage,
})
