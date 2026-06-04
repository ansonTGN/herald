import { useQuery } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { toast } from 'sonner'
import { useNavigate } from '@tanstack/react-router'
import { auditListQueryOptions } from '@/data/query-options'
import { PageHeader, ListPagination } from '@/components/shared'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import type { AuditSearchParams } from '@/lib/schemas/search-params'
import { AuditFilterBar, hasActiveFilters } from './audit-filter-bar'
import { AuditEventTable } from './audit-event-table'
import { AuditEventDetailSheet } from './audit-event-detail-sheet'
import { m } from '@/paraglide/messages'

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
      toast.error(error.message || m['audit.failed_to_load']())
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
      <PageHeader title={m['audit.page_title']()} headingTestId="audit-heading" />

      <AuditFilterBar
        filters={search}
        onFilterChange={handleFilterChange}
        onClearFilters={handleClearFilters}
      />

      <Card>
        <CardContent className="space-y-4">
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
              onRowClick={(event) => setSelectedEventId(event.id)}
              emptyMessage={
                hasActiveFilters(search) ? m['audit.no_matching_logs']() : m['audit.no_logs']()
              }
            />
          ) : null}
        </CardContent>
      </Card>

      {data && (
        <ListPagination
          page={data.page}
          pageSize={data.pageSize}
          total={data.total}
          onPageChange={handlePageChange}
          testIdPrefix="audit-pagination"
        />
      )}

      <AuditEventDetailSheet
        eventId={selectedEventId}
        realmId={realmId}
        onClose={() => setSelectedEventId(null)}
      />
    </div>
  )
}
