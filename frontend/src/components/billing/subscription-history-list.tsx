import { memo, useMemo } from 'react'
import { format } from 'date-fns'
import { Clock, User, ChevronRight, ArrowUpDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { HistoryEventBadge } from './history-event-badge'
import type { SubscriptionHistoryEventWithUser } from '@/types/billing'

interface SubscriptionHistoryListProps {
  events: SubscriptionHistoryEventWithUser[]
  loading?: boolean
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
  onSortChange,
  onEventClick,
  className,
}: SubscriptionHistoryListProps) {
  function handleSort(sortBy: string) {
    onSortChange?.(sortBy)
  }

  return (
    <div className={`rounded-md border ${className ?? ''}`} data-testid="subscription-history-list">
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
    </div>
  )
}
