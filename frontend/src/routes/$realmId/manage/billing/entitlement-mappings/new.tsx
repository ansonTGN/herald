import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { usePermission } from '@/hooks/use-permission'
import { useResolvedRealmId } from '@/lib/realm-routing'

const CreateEntitlementMappingPage = lazy(() =>
  import('@/components/billing/create-entitlement-mapping-page').then((m) => ({
    default: m.CreateEntitlementMappingPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/billing/entitlement-mappings/new')({
  component: NewEntitlementMappingRoute,
})

export function NewEntitlementMappingRoute() {
  const realmId = useResolvedRealmId()
  const { hasPermission } = usePermission()
  const canManagePoints = hasPermission('points.manage')

  return (
    <div className="container max-w-5xl mx-auto py-6 px-6">
      <Suspense
        fallback={
          <div
            className="flex items-center justify-center py-12"
            data-testid="create-mapping-form-loading"
          >
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <CreateEntitlementMappingPage realmId={realmId} canManagePoints={canManagePoints} />
      </Suspense>
    </div>
  )
}
