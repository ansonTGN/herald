import type { LucideIcon } from 'lucide-react'
import { Link } from '@tanstack/react-router'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'

interface StatsCardProps {
  title: string
  value: number | string
  description: string
  icon?: LucideIcon
  testId?: string
  linkTo?: string
  linkParams?: Record<string, string>
}

export function StatsCard({
  title,
  value,
  description,
  icon: Icon,
  testId,
  linkTo,
  linkParams,
}: StatsCardProps) {
  const cardContent = (
    <Card
      className={
        linkTo
          ? 'cursor-pointer transition-all duration-200 hover:shadow-md hover:border-primary/20'
          : undefined
      }
      data-testid={testId}
    >
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          {title}
        </CardTitle>
        {Icon && (
          <div className="flex size-8 items-center justify-center rounded-lg bg-primary/10">
            <Icon className="size-4 text-primary" />
          </div>
        )}
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold tracking-tight">{value}</div>
        <p className="mt-0.5 text-xs text-muted-foreground">{description}</p>
      </CardContent>
    </Card>
  )

  if (linkTo) {
    return (
      <Link to={linkTo} params={linkParams}>
        {cardContent}
      </Link>
    )
  }

  return cardContent
}
