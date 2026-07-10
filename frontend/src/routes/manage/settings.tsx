import { createFileRoute } from '@tanstack/react-router'
import { SettingsPage } from '@/routes/$realmId/manage/settings'

export const Route = createFileRoute('/manage/settings')({
  component: SettingsPage,
})
