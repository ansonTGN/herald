import { createFileRoute } from '@tanstack/react-router'
import { RevealApiKeyPage } from '@/routes/$realmId/manage/api-keys/reveal'

export const Route = createFileRoute('/manage/api-keys/reveal')({
  component: RevealApiKeyPage,
})
