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
      className={linkTo ? 'cursor-pointer transition-colors hover:bg-accent' : undefined}
      data-testid={testId}
    >
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        {Icon && <Icon className="h-4 w-4 text-muted-foreground" />}
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-bold">{value}</div>
        <p className="text-xs text-muted-foreground">{description}</p>
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
