import { createFileRoute, redirect } from '@tanstack/react-router'
import { useAuthStore } from '@/stores/auth-store'
import { auditSearchSchema, type AuditSearchParams } from '@/lib/schemas/search-params'
import { AuditPage } from '@/components/audit/audit-page'
import { useCurrentSearch, useResolvedRealmId } from '@/lib/realm-routing'

export const Route = createFileRoute('/$realmId/manage/audit/')({
  component: AuditRoute,
  validateSearch: (search) => auditSearchSchema.parse(search),
  loader: ({ params }) => {
    const urlRealmId = params.realmId
    const authRealmId = useAuthStore.getState().realmId

    if (authRealmId && urlRealmId !== authRealmId) {
      console.warn(
        `[Audit loader] Cross-realm access blocked - URL: ${urlRealmId}, Auth: ${authRealmId}`
      )
      throw redirect({
        to: '/$realmId/manage/audit',
        params: { realmId: authRealmId },
      })
    }

    return { urlRealmId }
  },
})

function AuditRoute() {
  const search = useCurrentSearch<AuditSearchParams>()
  const realmId = useResolvedRealmId()

  return <AuditPage realmId={realmId} search={search} />
}
