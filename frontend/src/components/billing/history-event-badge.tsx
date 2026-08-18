import { cn } from '@/lib/utils'
import type { SubscriptionHistoryEventType } from '@/types/billing'
import { getEventTypeLabel } from '@/types/billing'

interface HistoryEventBadgeProps {
  eventType: SubscriptionHistoryEventType
  size?: 'sm' | 'md' | 'lg'
  className?: string
}

const sizeClasses = {
  sm: 'px-2 py-0.5 text-xs',
  md: 'px-2.5 py-0.5 text-xs',
  lg: 'px-3 py-1 text-sm',
}

const colorClasses: Record<SubscriptionHistoryEventType, string> = {
  created: 'bg-success/10 text-success border-success/20',
  upgraded: 'bg-info/10 text-info border-info/20',
  downgraded: 'bg-warning/10 text-warning border-warning/20',
  canceled: 'bg-destructive/10 text-destructive border-destructive/20',
  renewed: 'bg-success/10 text-success border-success/20',
  reactivated: 'bg-info/10 text-info border-info/20',
  expired: 'bg-muted text-muted-foreground border-border',
  billing_period_changed: 'bg-info/10 text-info border-info/20',
  past_due: 'bg-warning/10 text-warning border-warning/20',
  disputed: 'bg-destructive/10 text-destructive border-destructive/20',
  payment_succeeded: 'bg-success/10 text-success border-success/20',
  payment_failed: 'bg-destructive/10 text-destructive border-destructive/20',
  invoice_created: 'bg-muted text-muted-foreground border-border',
}

export function HistoryEventBadge({ eventType, size = 'md', className }: HistoryEventBadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-md border font-medium',
        sizeClasses[size],
        colorClasses[eventType],
        className
      )}
      data-testid={`event-badge-${eventType}`}
    >
      {getEventTypeLabel(eventType)}
    </span>
  )
}
