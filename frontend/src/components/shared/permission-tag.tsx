import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

interface PermissionTagProps {
  permission: string // 格式: "resource.action"
  showDescription?: boolean
  className?: string
}

export function PermissionTag({
  permission,
  showDescription = false,
  className,
}: PermissionTagProps) {
  const parts = permission.split('.')
  const resource = parts[0] || permission
  const action = parts[1] || ''

  // 根据 action 类型选择颜色
  const getVariant = (): 'default' | 'destructive' | 'secondary' => {
    if (action === 'manage' || action === 'delete') return 'destructive'
    if (action === 'view' || action === 'list') return 'default'
    return 'secondary'
  }

  const variant = getVariant()

  return (
    <Badge
      variant={variant}
      className={cn('text-xs', className)}
      data-testid={`permission-tag-${permission.replace('.', '-')}`}
    >
      {permission}
      {showDescription && <span className="ml-1 text-muted-foreground">({resource})</span>}
    </Badge>
  )
}
