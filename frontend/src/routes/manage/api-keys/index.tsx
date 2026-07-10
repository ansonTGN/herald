import { createFileRoute } from '@tanstack/react-router'
import { ApiKeysPage } from '@/routes/$realmId/manage/api-keys/index'

export const Route = createFileRoute('/manage/api-keys/')({
  component: ApiKeysPage,
})
