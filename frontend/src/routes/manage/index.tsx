import { createFileRoute } from '@tanstack/react-router'
import { ManageDashboard } from '@/routes/$realmId/manage'

export const Route = createFileRoute('/manage/')({
  component: ManageDashboard,
})
