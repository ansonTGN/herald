import { createFileRoute } from '@tanstack/react-router'
import { ResetPasswordPage } from '@/routes/$realmId/auth/reset-password'
import { resetPasswordSearchSchema } from '@/lib/schemas/search-params'

export const Route = createFileRoute('/auth/reset-password')({
  component: ResetPasswordPage,
  validateSearch: (search) => resetPasswordSearchSchema.parse(search),
})
