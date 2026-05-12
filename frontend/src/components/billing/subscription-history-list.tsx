import { memo, useMemo } from 'react'
import { format } from 'date-fns'
import { Clock, User, ChevronRight, ArrowUpDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationNext,
  PaginationPrevious,
} from '@/components/ui/pagination'
import { HistoryEventBadge } from './history-event-badge'
import type { SubscriptionHistoryEventWithUser, PaginationInfo } from '@/types/billing'

interface SubscriptionHistoryListProps {
  events: SubscriptionHistoryEventWithUser[]
  loading?: boolean
  pagination?: PaginationInfo
  onPageChange?: (page: number) => void
  onSortChange?: (sortBy: string) => void
  onEventClick?: (event: SubscriptionHistoryEventWithUser) => void
  className?: string
}

interface HistoryTableRowProps {
  event: SubscriptionHistoryEventWithUser
  index: number
  onEventClick?: (event: SubscriptionHistoryEventWithUser) => void
}

const HistoryTableRow = memo(({ event, index, onEventClick }: HistoryTableRowProps) => {
  const timestamp = useMemo(() => format(new Date(event.timestamp), 'PPp'), [event.timestamp])

  return (
    <TableRow data-testid={`history-row-${index}`}>
      <TableCell className="font-medium">{timestamp}</TableCell>
      <TableCell>
        <HistoryEventBadge eventType={event.eventType} />
      </TableCell>
      <TableCell>
        <div className="space-y-1">
          <div className="text-sm font-medium">
            {event.subscription.plan?.title || 'Unknown Plan'}
          </div>
          <div className="text-xs text-muted-foreground">{event.subscription.status}</div>
        </div>
      </TableCell>
      <TableCell>
        {event.user ? (
          <div className="space-y-1">
            <div className="text-sm">{event.user.email}</div>
            <div className="text-xs text-muted-foreground">{event.user.id}</div>
          </div>
        ) : (
          <span className="text-sm text-muted-foreground">-</span>
        )}
      </TableCell>
      <TableCell>
        <span className="text-sm">{event.actor || '-'}</span>
      </TableCell>
      <TableCell className="text-right">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onEventClick?.(event)}
          data-testid={`view-details-${index}`}
        >
          <ChevronRight className="h-4 w-4" />
          <span className="sr-only">View details</span>
        </Button>
      </TableCell>
    </TableRow>
  )
})

HistoryTableRow.displayName = 'HistoryTableRow'

export function SubscriptionHistoryList({
  events,
  loading,
  pagination,
  onPageChange,
  onSortChange,
  onEventClick,
  className,
}: SubscriptionHistoryListProps) {
  function handleSort(sortBy: string) {
    onSortChange?.(sortBy)
  }

  return (
    <Card className={className} data-testid="subscription-history-list">
      <CardContent className="p-0">
        <div className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-auto p-0 font-medium"
                    onClick={() => handleSort('timestamp')}
                  >
                    <Clock className="mr-2 h-3.5 w-3.5" />
                    Timestamp
                    <ArrowUpDown className="ml-2 h-3.5 w-3.5" />
                  </Button>
                </TableHead>
                <TableHead>Event Type</TableHead>
                <TableHead>Subscription</TableHead>
                <TableHead>
                  <User className="mr-2 h-3.5 w-3.5 inline" />
                  User
                </TableHead>
                <TableHead>Actor</TableHead>
                <TableHead className="text-right">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={6} className="h-24 text-center">
                    Loading...
                  </TableCell>
                </TableRow>
              ) : events.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
                    No history events found
                  </TableCell>
                </TableRow>
              ) : (
                events.map((event, index) => (
                  <HistoryTableRow
                    key={event.id}
                    event={event}
                    index={index}
                    onEventClick={onEventClick}
                  />
                ))
              )}
            </TableBody>
          </Table>
        </div>

        {pagination && (
          <div className="flex items-center justify-between border-t px-4 py-4">
            <div className="text-sm text-muted-foreground">
              {pagination.totalCount > 0 ? (
                <>
                  Page {pagination.page} of {pagination.totalPages} ({pagination.totalCount} total)
                </>
              ) : (
                <>No results</>
              )}
            </div>
            {pagination.totalPages > 1 ? (
              <Pagination>
                <PaginationContent>
                  <PaginationItem>
                    <PaginationPrevious
                      onClick={() => onPageChange?.(pagination.page - 1)}
                      className={pagination.page <= 1 ? 'opacity-50 pointer-events-none' : ''}
                      data-testid="previous-page-button"
                    >
                      Previous
                    </PaginationPrevious>
                  </PaginationItem>
                  <PaginationItem>
                    <PaginationNext
                      onClick={() => onPageChange?.(pagination.page + 1)}
                      className={
                        pagination.page >= pagination.totalPages
                          ? 'opacity-50 pointer-events-none'
                          : ''
                      }
                      data-testid="next-page-button"
                    >
                      Next
                    </PaginationNext>
                  </PaginationItem>
                </PaginationContent>
              </Pagination>
            ) : null}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
