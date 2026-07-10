import { createFileRoute } from '@tanstack/react-router'
import { EditApiKeyPage } from '@/routes/$realmId/manage/api-keys/$apiKeyId.edit'

export const Route = createFileRoute('/manage/api-keys/$apiKeyId/edit')({
  component: EditApiKeyPage,
})
