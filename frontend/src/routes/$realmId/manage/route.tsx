import { createFileRoute } from '@tanstack/react-router'
import { AdminDashboardLayout } from '@/components/layouts/admin-dashboard-layout'

export const Route = createFileRoute('/$realmId/manage')({
  // NOTE: Authentication and permission checks are already handled by __root.tsx
  // The root route's loader checks for admin permissions before allowing access
  // No need to duplicate the check here
  component: AdminDashboardLayout,
})
