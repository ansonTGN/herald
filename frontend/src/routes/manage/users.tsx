import { createFileRoute } from '@tanstack/react-router'
import { usersSearchSchema } from '@/lib/schemas/search-params'
import { UsersPage } from '@/routes/$realmId/manage/users'

export const Route = createFileRoute('/manage/users')({
  validateSearch: (search) => usersSearchSchema.parse(search),
  component: UsersPage,
})
