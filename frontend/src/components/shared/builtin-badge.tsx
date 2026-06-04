import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import { m } from '@/paraglide/messages'

interface BuiltinBadgeProps {
  isBuiltin: boolean
  className?: string
}

export function BuiltinBadge({ isBuiltin, className }: BuiltinBadgeProps) {
  if (!isBuiltin) return null

  return (
    <Badge
      variant="secondary"
      className={cn('text-xs font-normal', className)}
      data-testid="builtin-badge"
    >
      {m['shared.builtin']()}
    </Badge>
  )
}
