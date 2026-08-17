import { createFileRoute } from '@tanstack/react-router'
import { NewEntitlementMappingRoute } from '@/routes/$realmId/manage/billing/entitlement-mappings/new'

export const Route = createFileRoute('/manage/billing/entitlement-mappings/new')({
  component: NewEntitlementMappingRoute,
})
