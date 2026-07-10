import { createFileRoute } from '@tanstack/react-router'
import { RolesPage } from '@/routes/$realmId/manage/roles'

export const Route = createFileRoute('/manage/roles')({
  component: RolesPage,
})
