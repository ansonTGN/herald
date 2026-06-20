import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Wallet } from 'lucide-react'
import type { DerivedBucketCard } from './user-points-view'
import { m } from '@/paraglide/messages'

interface PointsBalanceCardProps {
  card: DerivedBucketCard
  loading?: boolean
}

const BALANCES_BY_TYPE_KEYS = [
  'subscription',
  'topup',
  'registration',
  'freePeriodic',
  'granted',
] as const

export function PointsBalanceCard({ card, loading }: PointsBalanceCardProps) {
  if (loading) {
    return (
      <Card data-testid="points-balance-card">
        <CardHeader>
          <CardTitle>{m['points.balance_card_title']()}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-4">
            <div className="h-12 bg-muted rounded" />
          </div>
        </CardContent>
      </Card>
    )
  }

  const bucketTestId = card.bucketId
    ? `points-balance-card-${card.bucketId}`
    : 'points-balance-card'

  return (
    <Card data-testid={bucketTestId}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>{card.name ?? m['points.bucket_card_unnamed']()}</CardTitle>
          {card.enabled === false && (
            <Badge variant="secondary" data-testid={`points-balance-card-disabled-${card.bucketId ?? ''}`}>
              {m['points.bucket_card_disabled']()}
            </Badge>
          )}
        </div>
      </CardHeader>
      <CardContent>
        <div className="text-center">
          <div className="flex items-center justify-center gap-2 text-muted-foreground text-sm mb-2">
            <Wallet className="h-4 w-4" />
            <span>{m['points.balance_current']()}</span>
          </div>
          <div className="text-5xl font-bold" data-testid={`points-balance-total-${card.bucketId ?? ''}`}>
            {card.bucketTotal.toLocaleString()}
          </div>
          <div className="mt-4 flex flex-wrap justify-center gap-2">
            {BALANCES_BY_TYPE_KEYS.map((typeKey) => {
              const value = card.balancesByType[typeKey]
              if (!value) {
                return null
              }
              return (
                <Badge
                  key={typeKey}
                  variant="outline"
                  data-testid={`points-balance-type-${card.bucketId ?? ''}-${typeKey}`}
                >
                  {m[`points.balance_type_${typeKey}`]({ count: value.toLocaleString() })}
                </Badge>
              )
            })}
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
