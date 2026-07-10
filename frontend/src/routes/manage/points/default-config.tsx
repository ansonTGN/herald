import { createFileRoute } from '@tanstack/react-router'
import { RealmConfigPage } from '@/routes/$realmId/manage/points/default-config'

export const Route = createFileRoute('/manage/points/default-config')({
  component: RealmConfigPage,
})
