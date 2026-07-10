import { createFileRoute } from '@tanstack/react-router'
import { NewApiKeyPage } from '@/routes/$realmId/manage/api-keys/new'

export const Route = createFileRoute('/manage/api-keys/new')({
  component: NewApiKeyPage,
})
