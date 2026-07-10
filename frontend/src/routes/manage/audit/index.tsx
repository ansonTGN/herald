import { createFileRoute } from '@tanstack/react-router'
import { AuditPage } from '@/components/audit/audit-page'
import { auditSearchSchema, type AuditSearchParams } from '@/lib/schemas/search-params'
import { useCurrentSearch, useResolvedRealmId } from '@/lib/realm-routing'

export const Route = createFileRoute('/manage/audit/')({
  component: AuditRoute,
  validateSearch: (search) => auditSearchSchema.parse(search),
})

function AuditRoute() {
  const search = useCurrentSearch<AuditSearchParams>()
  const realmId = useResolvedRealmId()

  return <AuditPage realmId={realmId} search={search} />
}
