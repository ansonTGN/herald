import { createFileRoute } from '@tanstack/react-router'
import { AdminDashboardLayout } from '@/components/layouts/admin-dashboard-layout'

export const Route = createFileRoute('/manage')({
  component: AdminDashboardLayout,
})
