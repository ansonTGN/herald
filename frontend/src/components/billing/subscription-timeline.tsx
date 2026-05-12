import { useState, useCallback, useMemo, memo } from 'react'
import { format } from 'date-fns'
import { Clock, User, ChevronRight, Info } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { HistoryEventBadge } from './history-event-badge'
import type { SubscriptionHistoryEvent, SubscriptionStatus } from '@/types/billing'
import { SubscriptionStatusLabels } from '@/types/billing'
import { cn } from '@/lib/utils'

interface SubscriptionTimelineProps {
  events: SubscriptionHistoryEvent[]
  loading?: boolean
  onEventClick?: (event: SubscriptionHistoryEvent) => void
  className?: string
}

interface EventDetailDialogProps {
  event: SubscriptionHistoryEvent
  open: boolean
  onOpenChange: (open: boolean) => void
}

interface TimelineEventProps {
  event: SubscriptionHistoryEvent
  index: number
  onEventClick: (event: SubscriptionHistoryEvent) => void
}

const TimelineEvent = memo(({ event, index, onEventClick }: TimelineEventProps) => {
  const timestamp = useMemo(() => format(new Date(event.timestamp), 'PPp'), [event.timestamp])

  return (
    <div key={event.id} className="relative flex gap-4" data-testid={`timeline-event-${index}`}>
      {/* Timeline dot */}
      <div className="relative flex size-4 shrink-0 items-center justify-center">
        <div className="absolute -left-[3px] h-2 w-2 rounded-full bg-primary ring-4 ring-background" />
      </div>

      {/* Event content */}
      <Card className="flex-1 p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 space-y-2">
            <div className="flex items-center gap-2">
              <HistoryEventBadge eventType={event.eventType} />
            </div>

            <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
              <div className="flex items-center gap-1">
                <Clock className="h-3.5 w-3.5" />
                {timestamp}
              </div>

              {event.actor && (
                <div className="flex items-center gap-1">
                  <User className="h-3.5 w-3.5" />
                  {event.actor}
                </div>
              )}
            </div>

            {/* Change summary */}
            {event.changes && (
              <div className="text-sm">
                {event.changes.previousPlanId && event.changes.newPlanId && (
                  <span>
                    Plan changed from{' '}
                    <span className="font-medium text-foreground">
                      {event.changes.previousPlanId}
                    </span>{' '}
                    to{' '}
                    <span className="font-medium text-foreground">{event.changes.newPlanId}</span>
                  </span>
                )}
              </div>
            )}
          </div>

          <Button
            variant="ghost"
            size="sm"
            onClick={() => onEventClick(event)}
            className="shrink-0"
            data-testid={`view-event-details-${index}`}
          >
            <ChevronRight className="h-4 w-4" />
            <span className="sr-only">View details</span>
          </Button>
        </div>
      </Card>
    </div>
  )
})

TimelineEvent.displayName = 'TimelineEvent'

function EventDetailDialog({ event, open, onOpenChange }: EventDetailDialogProps) {
  return (
    <div
      className={cn(
        'fixed inset-0 z-50 flex items-center justify-center',
        open ? 'block' : 'hidden'
      )}
      data-testid="event-detail-dialog"
    >
      <div className="fixed inset-0 bg-black/50" onClick={() => onOpenChange(false)} />
      <Card className="relative z-50 mx-4 max-w-lg p-6">
        <div className="mb-4">
          <h3 className="text-lg font-semibold">Event Details</h3>
          <p className="text-sm text-muted-foreground">Event ID: {event.id}</p>
        </div>

        <div className="space-y-4">
          <div>
            <p className="text-sm font-medium text-muted-foreground">Event Type</p>
            <HistoryEventBadge eventType={event.eventType} />
          </div>

          <div>
            <p className="text-sm font-medium text-muted-foreground">Timestamp</p>
            <p className="text-sm">{format(new Date(event.timestamp), 'PPp')}</p>
          </div>

          {event.actor && (
            <div>
              <p className="text-sm font-medium text-muted-foreground">Actor</p>
              <p className="text-sm">{event.actor}</p>
            </div>
          )}

          {event.previousState && (
            <div>
              <p className="text-sm font-medium text-muted-foreground">Previous State</p>
              <div className="rounded-md bg-muted p-3">
                <p className="text-sm">
                  <span className="font-medium">Status:</span>{' '}
                  {SubscriptionStatusLabels[event.previousState.status as SubscriptionStatus] ||
                    event.previousState.status}
                </p>
                {event.previousState.planId && (
                  <p className="text-sm">
                    <span className="font-medium">Plan:</span> {event.previousState.planId}
                  </p>
                )}
                {event.previousState.billingPeriod && (
                  <p className="text-sm">
                    <span className="font-medium">Billing:</span>{' '}
                    {event.previousState.billingPeriod}
                  </p>
                )}
              </div>
            </div>
          )}

          {event.newState && (
            <div>
              <p className="text-sm font-medium text-muted-foreground">New State</p>
              <div className="rounded-md bg-muted p-3">
                <p className="text-sm">
                  <span className="font-medium">Status:</span>{' '}
                  {SubscriptionStatusLabels[event.newState.status as SubscriptionStatus] ||
                    event.newState.status}
                </p>
                {event.newState.planId && (
                  <p className="text-sm">
                    <span className="font-medium">Plan:</span> {event.newState.planId}
                  </p>
                )}
                {event.newState.billingPeriod && (
                  <p className="text-sm">
                    <span className="font-medium">Billing:</span> {event.newState.billingPeriod}
                  </p>
                )}
              </div>
            </div>
          )}

          {event.changes && (
            <div>
              <p className="text-sm font-medium text-muted-foreground">Changes</p>
              <div className="rounded-md bg-muted p-3">
                {event.changes.changedFields && event.changes.changedFields.length > 0 && (
                  <div>
                    <p className="mb-2 text-xs font-medium">Changed Fields:</p>
                    <div className="flex flex-wrap gap-1">
                      {event.changes.changedFields.map((field) => (
                        <span key={field} className="rounded bg-background px-2 py-0.5 text-xs">
                          {field}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        <div className="mt-6 flex justify-end">
          <Button onClick={() => onOpenChange(false)}>Close</Button>
        </div>
      </Card>
    </div>
  )
}

export function SubscriptionTimeline({
  events,
  loading,
  onEventClick,
  className,
}: SubscriptionTimelineProps) {
  const [selectedEvent, setSelectedEvent] = useState<SubscriptionHistoryEvent | null>(null)

  const handleEventClick = useCallback(
    (event: SubscriptionHistoryEvent) => {
      setSelectedEvent(event)
      onEventClick?.(event)
    },
    [onEventClick]
  )

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8" data-testid="timeline-loading">
        <div className="text-sm text-muted-foreground">Loading history...</div>
      </div>
    )
  }

  if (events.length === 0) {
    return (
      <div className="flex items-center justify-center py-8" data-testid="timeline-empty">
        <div className="text-center">
          <Info className="mx-auto mb-2 h-12 w-12 text-muted-foreground" />
          <p className="text-sm text-muted-foreground">No history events found</p>
        </div>
      </div>
    )
  }

  return (
    <div className={className} data-testid="subscription-timeline">
      <div className="relative space-y-6 before:absolute before:left-2 before:top-2 before:h-[calc(100%-1rem)] before:w-px before:bg-border">
        {events.map((event, index) => (
          <TimelineEvent
            key={event.id}
            event={event}
            index={index}
            onEventClick={handleEventClick}
          />
        ))}
      </div>

      {selectedEvent && (
        <EventDetailDialog
          event={selectedEvent}
          open={!!selectedEvent}
          onOpenChange={(open) => !open && setSelectedEvent(null)}
        />
      )}
    </div>
  )
}
