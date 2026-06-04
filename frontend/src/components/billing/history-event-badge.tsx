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
  created: 'bg-green-100 text-green-800 border-green-200',
  upgraded: 'bg-blue-100 text-blue-800 border-blue-200',
  downgraded: 'bg-orange-100 text-orange-800 border-orange-200',
  canceled: 'bg-red-100 text-red-800 border-red-200',
  renewed: 'bg-green-100 text-green-800 border-green-200',
  reactivated: 'bg-purple-100 text-purple-800 border-purple-200',
  expired: 'bg-gray-100 text-gray-800 border-gray-200',
  billing_period_changed: 'bg-cyan-100 text-cyan-800 border-cyan-200',
  past_due: 'bg-yellow-100 text-yellow-800 border-yellow-200',
  disputed: 'bg-red-100 text-red-800 border-red-200',
  payment_succeeded: 'bg-green-100 text-green-800 border-green-200',
  payment_failed: 'bg-red-100 text-red-800 border-red-200',
  invoice_created: 'bg-gray-100 text-gray-800 border-gray-200',
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
