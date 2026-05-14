import { useQuery } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { useNavigate } from '@tanstack/react-router'
import { auditListQueryOptions } from '@/data/query-options'
import { PageHeader } from '@/components/shared'
import { Skeleton } from '@/components/ui/skeleton'
import type { AuditSearchParams } from '@/lib/schemas/search-params'
import { AuditFilterBar, hasActiveFilters } from './audit-filter-bar'
import { AuditEventTable } from './audit-event-table'
import { AuditEventDetailSheet } from './audit-event-detail-sheet'

interface AuditPageProps {
  realmId: string
  search: AuditSearchParams
}

export function AuditPage({ realmId, search }: AuditPageProps) {
  const navigate = useNavigate()
  const [selectedEventId, setSelectedEventId] = useState<string | null>(null)

  const { data, isLoading, error } = useQuery(
    auditListQueryOptions(realmId, {
      page: search.page,
      pageSize: search.pageSize,
      category: search.category,
      action: search.action,
      actorId: search.actorId,
      startTime: search.startTime,
      endTime: search.endTime,
    })
  )

  useEffect(() => {
    if (error) {
      toast.error(error.message || 'Failed to load audit logs')
    }
  }, [error])

  function handleFilterChange(filters: Partial<AuditSearchParams>) {
    navigate({
      to: '/$realmId/manage/audit',
      params: { realmId },
      search: {
        ...search,
        ...filters,
        page: 0,
      },
    })
  }

  function handleClearFilters() {
    navigate({
      to: '/$realmId/manage/audit',
      params: { realmId },
      search: {},
    })
  }

  function handlePageChange(page: number) {
    navigate({
      to: '/$realmId/manage/audit',
      params: { realmId },
      search: { ...search, page },
    })
  }

  return (
    <div data-testid="audit-page" className="space-y-6">
      <PageHeader
        title="Audit Log"
        description="View and filter audit events for this realm"
        headingTestId="audit-heading"
      />

      <AuditFilterBar
        filters={search}
        onFilterChange={handleFilterChange}
        onClearFilters={handleClearFilters}
      />

      {isLoading ? (
        <div data-testid="audit-table-loading" className="space-y-3">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : data ? (
        <AuditEventTable
          data={data.items}
          total={data.total}
          page={data.page}
          pageSize={data.pageSize}
          onRowClick={(event) => setSelectedEventId(event.id)}
          onPageChange={handlePageChange}
          emptyMessage={
            hasActiveFilters(search)
              ? 'No matching audit logs. Try adjusting your filters.'
              : 'No audit logs yet.'
          }
        />
      ) : null}

      <AuditEventDetailSheet
        eventId={selectedEventId}
        realmId={realmId}
        onClose={() => setSelectedEventId(null)}
      />
    </div>
  )
}
