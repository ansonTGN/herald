import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { TrendingUp, TrendingDown, Wallet } from 'lucide-react'
import type { PointsAccountResponse } from '@/lib/api-generated'

interface PointsBalanceCardProps {
  account: PointsAccountResponse | null
  loading?: boolean
}

export function PointsBalanceCard({ account, loading }: PointsBalanceCardProps) {
  if (loading) {
    return (
      <Card data-testid="points-balance-card">
        <CardHeader>
          <CardTitle>Points Balance</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-4">
            <div className="h-12 bg-muted rounded" />
            <div className="grid grid-cols-2 gap-4">
              <div className="h-8 bg-muted rounded" />
              <div className="h-8 bg-muted rounded" />
            </div>
          </div>
        </CardContent>
      </Card>
    )
  }

  if (!account) {
    return null
  }

  const statusConfig = {
    active: { label: 'Active', color: 'text-green-600 bg-green-50' },
    frozen: { label: 'Frozen', color: 'text-red-600 bg-red-50' },
    closed: { label: 'Closed', color: 'text-gray-600 bg-gray-50' },
    unknown: { label: 'Unknown', color: 'text-yellow-600 bg-yellow-50' },
  } as const

  const status = statusConfig[account.status as keyof typeof statusConfig] || statusConfig.unknown

  return (
    <Card data-testid="points-balance-card">
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle>Points Balance</CardTitle>
          <span
            className={`px-2 py-1 rounded-full text-xs font-medium ${status.color}`}
            data-testid="points-account-status"
          >
            {status.label}
          </span>
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Current Balance */}
        <div className="text-center">
          <div className="flex items-center justify-center gap-2 text-muted-foreground text-sm mb-2">
            <Wallet className="h-4 w-4" />
            <span>Current Balance</span>
          </div>
          <div className="text-5xl font-bold" data-testid="points-balance">
            {account.balance.toLocaleString()}
          </div>
          <div className="text-sm text-muted-foreground mt-1">{account.unit}</div>
        </div>

        {/* Stats */}
        <div className="grid grid-cols-2 gap-4">
          <div className="bg-muted/50 rounded-lg p-4">
            <div className="flex items-center gap-2 text-muted-foreground text-sm mb-1">
              <TrendingUp className="h-4 w-4 text-green-600" />
              <span>Total Recharged</span>
            </div>
            <div
              className="text-2xl font-semibold text-green-600"
              data-testid="points-total-recharged"
            >
              {account.totalRecharged.toLocaleString()}
            </div>
          </div>
          <div className="bg-muted/50 rounded-lg p-4">
            <div className="flex items-center gap-2 text-muted-foreground text-sm mb-1">
              <TrendingDown className="h-4 w-4 text-red-600" />
              <span>Total Consumed</span>
            </div>
            <div
              className="text-2xl font-semibold text-red-600"
              data-testid="points-total-consumed"
            >
              {account.totalConsumed.toLocaleString()}
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
