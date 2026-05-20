import type { ReactNode } from 'react'
import { Button } from '@/components/ui/button'
import { Plus } from 'lucide-react'

export interface PageHeaderProps {
  title: string
  subtitle?: string
  headingTestId?: string
  action?: {
    label: string
    onClick: () => void
    testId: string
    show?: boolean
    icon?: ReactNode
  }
  className?: string
}

/**
 * Standardized page header component for list pages.
 * Provides consistent layout for page title and action button.
 *
 * @example
 * ```tsx
 * <PageHeader
 *   title="Roles"
 *   action={{
 *     label: "Add Role",
 *     onClick: () => setDialogOpen(true),
 *     testId: "role-create-button"
 *   }}
 * />
 * ```
 */
export function PageHeader({
  title,
  subtitle,
  headingTestId,
  action,
  className = '',
}: PageHeaderProps) {
  return (
    <div className={`flex items-center justify-between ${className}`}>
      <div>
        <h1
          className="text-xl font-semibold"
          data-testid={headingTestId ?? `${title.toLowerCase()}-heading`}
        >
          {title}
        </h1>
        {subtitle && <p className="text-sm text-muted-foreground">{subtitle}</p>}
      </div>
      {action && action.show !== false && (
        <Button onClick={action.onClick} data-testid={action.testId}>
          {action.icon ?? <Plus className="mr-2 h-4 w-4" />}
          {action.label}
        </Button>
      )}
    </div>
  )
}
