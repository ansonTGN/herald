import { createFileRoute } from '@tanstack/react-router'
import { LoginPage } from '@/routes/$realmId/auth/login'
import { loginSearchSchema } from '@/lib/schemas/search-params'

export const Route = createFileRoute('/auth/login')({
  component: LoginPage,
  validateSearch: (search) => loginSearchSchema.parse(search),
})
