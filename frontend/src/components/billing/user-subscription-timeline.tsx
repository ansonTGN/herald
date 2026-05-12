import { useState, useMemo } from 'react'
import { format } from 'date-fns'
import { Clock, ChevronDown, ChevronUp, ArrowUp } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { HistoryEventBadge } from './history-event-badge'
import type { SubscriptionHistoryEvent } from '@/types/billing'

interface UserSubscriptionTimelineProps {
  events: SubscriptionHistoryEvent[]
  loading?: boolean
}

interface EventCardProps {
  event: SubscriptionHistoryEvent
  isExpanded: boolean
  onToggle: () => void
}

function EventCard({ event, isExpanded, onToggle }: EventCardProps) {
  const timestamp = format(new Date(event.timestamp), 'PPp')

  return (
    <div
      className="border-l-2 border-muted pl-6 pb-8 last:pb-0 relative"
      data-testid={`timeline-event-${event.id}`}
    >
      {/* Timeline dot */}
      <div className="absolute -left-[5px] top-6 w-2.5 h-2.5 rounded-full bg-primary" />

      <div className="space-y-3">
        <div className="flex items-center justify-between flex-wrap gap-2">
          <div className="flex items-center gap-2">
            <HistoryEventBadge eventType={event.eventType} />
            <span className="text-sm text-muted-foreground">{timestamp}</span>
          </div>
          {(event.previousState || event.newState) && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onToggle}
              className="h-7 px-2 text-xs"
              data-testid={`toggle-event-details-${event.id}`}
            >
              {isExpanded ? (
                <>
                  <ChevronUp className="h-3 w-3 mr-1" />
                  Hide Details
                </>
              ) : (
                <>
                  <ChevronDown className="h-3 w-3 mr-1" />
                  Show Details
                </>
              )}
            </Button>
          )}
        </div>

        <div className="space-y-1">
          {event.actor && (
            <p className="text-sm">
              <span className="text-muted-foreground">Actor: </span>
              {event.actor}
            </p>
          )}

          {event.changes && event.changes.changedFields && (
            <div className="text-sm">
              <span className="text-muted-foreground">Changed: </span>
              <span className="font-medium">{event.changes.changedFields.join(', ')}</span>
            </div>
          )}

          {/* Expandable details */}
          {isExpanded && (event.previousState || event.newState) && (
            <div className="mt-3 space-y-3 p-3 bg-muted/30 rounded-lg">
              {event.previousState && (
                <div className="space-y-2">
                  <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                    Previous State
                  </p>
                  <div className="space-y-1 text-sm">
                    <div>
                      <span className="text-muted-foreground">Status: </span>
                      {event.previousState.status}
                    </div>
                    <div>
                      <span className="text-muted-foreground">Tier: </span>
                      {event.previousState.tier}
                    </div>
                    {event.previousState.currentPeriodStart && (
                      <div>
                        <span className="text-muted-foreground">Period: </span>
                        {event.previousState.currentPeriodEnd &&
                          `${new Date(event.previousState.currentPeriodStart).toLocaleDateString()} - ${new Date(event.previousState.currentPeriodEnd).toLocaleDateString()}`}
                      </div>
                    )}
                  </div>
                </div>
              )}

              {event.newState && (
                <div className="space-y-2">
                  <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                    New State
                  </p>
                  <div className="space-y-1 text-sm">
                    <div>
                      <span className="text-muted-foreground">Status: </span>
                      {event.newState.status}
                    </div>
                    <div>
                      <span className="text-muted-foreground">Tier: </span>
                      {event.newState.tier}
                    </div>
                    {event.newState.currentPeriodStart && (
                      <div>
                        <span className="text-muted-foreground">Period: </span>
                        {event.newState.currentPeriodEnd &&
                          `${new Date(event.newState.currentPeriodStart).toLocaleDateString()} - ${new Date(event.newState.currentPeriodEnd).toLocaleDateString()}`}
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* Plan change indicator */}
              {event.changes && (event.changes.previousPlanId || event.changes.newPlanId) && (
                <div className="flex items-center gap-2 text-sm pt-2 border-t border-border/50">
                  {event.changes.previousPlanId && event.changes.newPlanId ? (
                    <>
                      <ArrowUp className="h-4 w-4 text-blue-500" />
                      <span className="text-muted-foreground">Plan changed</span>
                    </>
                  ) : event.changes.newPlanId ? (
                    <>
                      <ArrowUp className="h-4 w-4 text-blue-500" />
                      <span className="text-muted-foreground">Plan assigned</span>
                    </>
                  ) : null}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export function UserSubscriptionTimeline({
  events,
  loading = false,
}: UserSubscriptionTimelineProps) {
  const [expandedEvents, setExpandedEvents] = useState<Set<string>>(new Set())

  const toggleEvent = (eventId: string) => {
    setExpandedEvents((prev) => {
      const next = new Set(prev)
      if (next.has(eventId)) {
        next.delete(eventId)
      } else {
        next.add(eventId)
      }
      return next
    })
  }

  // Sort events by timestamp descending (newest first)
  const sortedEvents = useMemo(
    () =>
      [...events].sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()),
    [events]
  )

  if (loading) {
    return (
      <Card data-testid="subscription-timeline-loading">
        <CardContent className="py-8">
          <div className="flex items-center justify-center text-muted-foreground">
            <Clock className="h-5 w-5 mr-2 animate-spin" />
            Loading history...
          </div>
        </CardContent>
      </Card>
    )
  }

  if (events.length === 0) {
    return (
      <Card data-testid="subscription-timeline-empty">
        <CardContent className="py-8">
          <div className="text-center text-muted-foreground">No history available</div>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card data-testid="subscription-timeline">
      <CardHeader>
        <CardTitle className="text-xl">Subscription History</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="space-y-0">
          {sortedEvents.map((event) => (
            <EventCard
              key={event.id}
              event={event}
              isExpanded={expandedEvents.has(event.id)}
              onToggle={() => toggleEvent(event.id)}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  )
}
