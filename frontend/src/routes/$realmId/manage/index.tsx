import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { LayoutDashboard, Users, UserPlus, Activity } from 'lucide-react'
import { dashboardStatsQueryOptions } from '@/data/query-options'
import { StatsCard } from '@/components/dashboard/stats-card'
import { AuthTrendChart } from '@/components/dashboard/auth-trend-chart'
import { QuickNav } from '@/components/dashboard/quick-nav'
import { Skeleton } from '@/components/ui/skeleton'

export const Route = createFileRoute('/$realmId/manage/')({
  component: ManageDashboard,
})

function ManageDashboard() {
  const { realmId } = Route.useParams()
  const { data, isLoading, isError, error, refetch } = useQuery(dashboardStatsQueryOptions(realmId))

  const userStats = data?.userStats
  const authTrend = data?.authTrend ?? []

  return (
    <div className="container mx-auto py-6 space-y-6">
      <div className="flex items-center gap-3">
        <div className="flex size-10 items-center justify-center rounded-xl bg-primary/10">
          <LayoutDashboard className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-xl font-semibold tracking-tight">Dashboard</h1>
          <p className="text-xs text-muted-foreground">Overview of your authentication system</p>
        </div>
      </div>

      {isError ? (
        <div
          className="rounded-lg border border-destructive/50 bg-destructive/10 p-6 text-center"
          data-testid="dashboard-error"
        >
          <p className="text-destructive mb-3">
            {error instanceof Error ? error.message : 'Failed to load dashboard data'}
          </p>
          <button
            onClick={() => refetch()}
            className="rounded-md bg-primary px-4 py-2 text-sm text-primary-foreground hover:bg-primary/90"
            data-testid="dashboard-retry-button"
          >
            Retry
          </button>
        </div>
      ) : (
        <>
          {/* Stats Cards */}
          <div className="grid gap-4 md:grid-cols-3" data-testid="dashboard-stats-row">
            {isLoading ? (
              <>
                <Skeleton className="h-[120px] rounded-xl" />
                <Skeleton className="h-[120px] rounded-xl" />
                <Skeleton className="h-[120px] rounded-xl" />
              </>
            ) : (
              <>
                <StatsCard
                  title="Total Users"
                  value={userStats?.totalUsers ?? 0}
                  description="users"
                  icon={Users}
                  testId="dashboard-total-users-card"
                  linkTo="/$realmId/manage/users"
                  linkParams={{ realmId }}
                />
                <StatsCard
                  title="New Users"
                  value={userStats?.newUsers ?? 0}
                  description="past 7 days"
                  icon={UserPlus}
                  testId="dashboard-new-users-card"
                />
                <StatsCard
                  title="Active Users"
                  value={userStats?.activeUsers ?? 0}
                  description="past 7 days"
                  icon={Activity}
                  testId="dashboard-active-users-card"
                />
              </>
            )}
          </div>

          {/* Auth Trend Chart */}
          {isLoading ? (
            <Skeleton className="h-[350px] rounded-xl" data-testid="dashboard-chart-skeleton" />
          ) : (
            <AuthTrendChart data={authTrend} testId="dashboard-auth-trend-chart" />
          )}

          {/* Quick Navigation */}
          <QuickNav realmId={realmId} testId="dashboard-quick-nav" />
        </>
      )}
    </div>
  )
}
