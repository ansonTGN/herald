import { createFileRoute } from '@tanstack/react-router'
import { ClientAppsPage } from '@/routes/$realmId/manage/client-apps/index'

export const Route = createFileRoute('/manage/client-apps/')({
  component: ClientAppsPage,
})
