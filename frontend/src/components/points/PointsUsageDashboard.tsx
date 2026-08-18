import { Alert, AlertDescription } from '@/components/ui/alert'
import { Badge } from '@/components/ui/badge'
import { Skeleton } from '@/components/ui/skeleton'
import { m } from '@/paraglide/messages'
import type { QuotaWindowViewResponse } from '@/lib/api-generated'
import type { DerivedBucketCard } from './user-points-view'

interface PointsUsageDashboardProps {
  card: DerivedBucketCard
  loading?: boolean
}

// Windows within this fraction of their limit are flagged "near limit".
const NEAR_LIMIT_THRESHOLD = 0.9

/**
 * Format a `resetsAt` ISO timestamp as a coarse human duration for the
 * `points.window_resets_in` interpolation (e.g. "3h", "2d"). Returns `null`
 * when the timestamp is missing or already in the past.
 */
function formatResetsDuration(resetsAt?: string | null): string | null {
  if (!resetsAt) {
    return null
  }
  const target = Date.parse(resetsAt)
  if (Number.isNaN(target)) {
    return null
  }
  const ms = target - Date.now()
  if (ms <= 0) {
    return null
  }
  const hours = ms / (1000 * 60 * 60)
  if (hours < 1) {
    return `${Math.max(1, Math.round(hours * 60))}m`
  }
  if (hours < 48) {
    return `${Math.round(hours)}h`
  }
  return `${Math.round(hours / 24)}d`
}

function windowBarColor(
  window: QuotaWindowViewResponse
): 'bg-primary' | 'bg-warning' | 'bg-destructive' {
  if (window.exhausted) {
    return 'bg-destructive'
  }
  if (window.isTightest) {
    return 'bg-warning'
  }
  return 'bg-primary'
}

export function PointsUsageDashboard({ card, loading }: PointsUsageDashboardProps) {
  if (loading) {
    return (
      <div data-testid="points-usage-dashboard" className="space-y-3">
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-16 w-full" />
      </div>
    )
  }

  const bucketId = card.bucketId ?? ''
  const spendableTotal = card.bucketTotal
  const spendableFromQuota = card.spendableFromQuota ?? 0
  const spendableFromPool = card.spendableFromPool ?? 0
  const windows = card.quotaWindows ?? []
  const hasWindows = windows.length > 0

  const anyWindowExhausted = windows.some((window) => window.exhausted)
  const empty = !hasWindows && spendableFromQuota === 0 && spendableFromPool === 0
  // `insufficient` (bucket had a balance model but is now drained) excludes
  // the empty state, so a brand-new user sees the onboarding empty-state
  // instead of a contradictory "transaction rejected" alert with no
  // transaction in progress.
  const insufficient = !empty && spendableTotal <= 0 && !anyWindowExhausted
  const overspendTopup = anyWindowExhausted && spendableFromPool > 0 && spendableTotal > 0

  const sortedWindows = [...windows].sort((a, b) => {
    if (a.exhausted !== b.exhausted) return a.exhausted ? -1 : 1
    if (a.isTightest !== b.isTightest) return a.isTightest ? -1 : 1
    return b.limit - a.limit
  })

  return (
    <div className="space-y-4" data-testid={`points-usage-dashboard-${bucketId}`}>
      {/* Tightest constraint alert (quota is the limiting factor) */}
      {spendableFromQuota >= 0 && hasWindows && (
        <Alert className="border-warning/50 text-warning ">
          <AlertDescription>{m['points.tightest_constraint']()}</AlertDescription>
        </Alert>
      )}

      {/* Key state alerts */}
      {empty && (
        <Alert
          variant="default"
          className="border-muted text-muted-foreground"
          data-testid="points-empty-state"
        >
          <AlertDescription>{m['points.empty_state']()}</AlertDescription>
        </Alert>
      )}
      {anyWindowExhausted && (
        <Alert variant="destructive" data-testid="points-window-exhausted-alert">
          <AlertDescription>{m['points.window_exhausted']()}</AlertDescription>
        </Alert>
      )}
      {overspendTopup && (
        <Alert
          className="border-warning/50 text-warning"
          data-testid="points-overspend-topup-alert"
        >
          <AlertDescription>{m['points.overspend_topup_alert']()}</AlertDescription>
        </Alert>
      )}
      {insufficient && (
        <Alert variant="destructive" data-testid="points-insufficient-alert">
          <AlertDescription>{m['points.insufficient_alert']()}</AlertDescription>
        </Alert>
      )}

      {/* Per-window rows */}
      {hasWindows && (
        <div className="space-y-4">
          {sortedWindows.map((window) => {
            const winKey = window.key
            const limit = window.limit
            const used = window.used
            const remaining = window.remaining
            const fillPct = limit > 0 ? Math.min(100, (remaining / limit) * 100) : 0
            const usedPct = limit > 0 ? used / limit : 0
            const nearLimit = !window.exhausted && usedPct >= NEAR_LIMIT_THRESHOLD
            const resetsDuration = formatResetsDuration(window.resetsAt)
            const barColor = windowBarColor(window)

            return (
              <div
                key={winKey}
                data-testid={`points-window-row-${bucketId}-${winKey}`}
                className={
                  window.exhausted
                    ? 'rounded-lg border border-destructive/50 bg-destructive/5 p-3 space-y-2'
                    : 'rounded-lg border p-3 space-y-2'
                }
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium">{winKey}</span>
                  <div className="flex flex-wrap gap-1">
                    {window.isTightest && (
                      <Badge variant="secondary">{m['points.tightest_constraint']()}</Badge>
                    )}
                    {window.exhausted && (
                      <Badge variant="destructive">{m['points.window_exhausted']()}</Badge>
                    )}
                    {nearLimit && (
                      <Badge variant="outline" className="border-warning text-warning ">
                        {m['points.window_near_limit']()}
                      </Badge>
                    )}
                  </div>
                </div>

                {/* Custom progress bar (no new deps) */}
                <div
                  className="h-2 w-full overflow-hidden rounded-full bg-muted"
                  data-testid={`points-window-bar-${bucketId}-${winKey}`}
                  role="progressbar"
                  aria-valuenow={Math.round(fillPct)}
                  aria-valuemin={0}
                  aria-valuemax={100}
                >
                  <div
                    className={`h-full rounded-full ${barColor}`}
                    style={{ width: `${fillPct}%` }}
                  />
                </div>

                <div className="text-muted-foreground flex items-center justify-between text-sm">
                  <span>
                    {remaining.toLocaleString()} / {limit.toLocaleString()} ·{' '}
                    {used.toLocaleString()}
                  </span>
                  {resetsDuration && (
                    <span>
                      {m['points.window_resets_in']({
                        duration: resetsDuration,
                      })}
                    </span>
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}

      {/* Pool balance summary (topup/registration/granted) */}
      {spendableFromPool > 0 && (
        <div className="border-t pt-3">
          <div className="text-muted-foreground text-sm">{m['points.balance_pool']()}</div>
          <div className="text-2xl font-semibold">{spendableFromPool.toLocaleString()}</div>
        </div>
      )}
    </div>
  )
}
