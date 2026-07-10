import { createFileRoute } from '@tanstack/react-router'
import { PermissionsPage } from '@/routes/$realmId/manage/permissions'

export const Route = createFileRoute('/manage/permissions')({
  component: PermissionsPage,
})
