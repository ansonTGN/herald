import { createFileRoute } from '@tanstack/react-router'
import { NewClientAppPage } from '@/routes/$realmId/manage/client-apps/new'

export const Route = createFileRoute('/manage/client-apps/new')({
  component: NewClientAppPage,
})
