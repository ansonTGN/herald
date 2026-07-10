import { createFileRoute } from '@tanstack/react-router'
import { RegisterPage } from '@/routes/$realmId/auth/register'

export const Route = createFileRoute('/auth/register')({
  component: RegisterPage,
})
