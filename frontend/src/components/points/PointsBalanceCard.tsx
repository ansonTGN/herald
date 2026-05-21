import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Wallet } from 'lucide-react'
import type { PointsWalletResponse } from '@/lib/api-generated'

interface PointsBalanceCardProps {
  account: PointsWalletResponse | null
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
            data-testid="points-wallet-status"
          >
            {status.label}
          </span>
        </div>
      </CardHeader>
      <CardContent>
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
      </CardContent>
    </Card>
  )
}
