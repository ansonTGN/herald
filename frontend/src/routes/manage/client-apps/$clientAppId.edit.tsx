import { createFileRoute } from '@tanstack/react-router'
import { EditClientAppPage } from '@/routes/$realmId/manage/client-apps/$clientAppId.edit'

export const Route = createFileRoute('/manage/client-apps/$clientAppId/edit')({
  component: EditClientAppPage,
})
