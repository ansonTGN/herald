import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { format, formatDistanceToNow } from 'date-fns'
import { zhCN } from 'date-fns/locale'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Input } from '@/components/ui/input'
import { Alert, AlertDescription } from '@/components/ui/alert'
import {
  Loader2,
  Users,
  UserCheck,
  Gift,
  Calendar,
  TrendingUp,
  Download,
  RefreshCw,
} from 'lucide-react'
import { freeUserStatsQueryOptions } from '@/data/query-options'
import { useState, useEffect } from 'react'

export const Route = createFileRoute('/$realmId/manage/points/free-stats')({
  component: FreeUserStatsPage,
  validateSearch: (search: Record<string, unknown>) => ({
    startDate: typeof search.startDate === 'string' ? search.startDate : undefined,
    endDate: typeof search.endDate === 'string' ? search.endDate : undefined,
  }),
})

function FreeUserStatsPage() {
  const { realmId } = Route.useParams()
  const search = Route.useSearch()
  const navigate = Route.useNavigate()

  // Get date range from search params
  // 从 URL 搜索参数中提取日期范围，用于筛选特定时间段的统计数据
  const startDate = search?.startDate
  const endDate = search?.endDate

  // State for relative time display
  // 用于显示相对时间（如"5分钟前"）
  const [relativeTime, setRelativeTime] = useState<string>('')

  // Fetch statistics
  // 获取免费用户统计数据，支持：
  // 1. 自动刷新（每5分钟）
  // 2. 日期范围筛选
  // 3. 错误重试（客户端错误不重试）
  // 4. 缓存管理（2分钟过期）
  const {
    data: stats,
    isLoading,
    error,
    refetch,
    isRefetching,
  } = useQuery(
    freeUserStatsQueryOptions(realmId, {
      startDate,
      endDate,
    })
  )

  // Update relative time every minute
  // 每分钟更新一次相对时间显示
  useEffect(() => {
    if (!stats) return

    const updateRelativeTime = () => {
      const lastUpdated = new Date(stats.lastUpdatedAt)
      const timeAgo = formatDistanceToNow(lastUpdated, {
        addSuffix: true,
        locale: zhCN,
      })
      setRelativeTime(timeAgo)
    }

    updateRelativeTime()
    const interval = setInterval(updateRelativeTime, 60000) // Update every minute

    return () => clearInterval(interval)
  }, [stats])

  // Handle date range change
  // 更新 URL 搜索参数以触发数据重新获取
  // 这样可以保持状态，支持浏览器前进/后退
  const handleDateChange = (field: 'startDate' | 'endDate', value: string) => {
    navigate({
      search: {
        ...search,
        [field]: value || undefined, // 空值转换为 undefined 以移除参数
      },
    })
  }

  // Export to CSV
  // 导出当前统计数据为 CSV 文件
  // 文件名包含 realm ID 和当前日期，便于归档和追踪
  const handleExport = () => {
    if (!stats) return

    // Build CSV content
    const csvContent = [
      ['Metric', 'Value'],
      ['Total Free Users', stats.totalFreeUsers.toString()],
      ['Active Free Users', stats.activeFreeUsers.toString()],
      ['Total Registration Bonus', stats.totalRegistrationBonusGranted.toString()],
      ['Total Periodic Points Granted', stats.totalPeriodicPointsGranted.toString()],
      ['Avg Periodic Points/User', stats.averagePeriodicPointsPerUser.toFixed(2)],
      ['Upgrade Rate', `${stats.upgradeRate.toFixed(2)}%`],
      ['Last Updated', stats.lastUpdatedAt],
    ]
      .map((row) => row.join(','))
      .join('\n')

    // 创建下载链接并触发下载
    const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
    const link = document.createElement('a')
    const url = URL.createObjectURL(blob)
    link.setAttribute('href', url)
    link.setAttribute(
      'download',
      `free-user-stats-${realmId}-${format(new Date(), 'yyyy-MM-dd')}.csv`
    )
    link.style.visibility = 'hidden'
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-8">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    )
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertDescription>Failed to load stats: {(error as Error).message}</AlertDescription>
      </Alert>
    )
  }

  if (!stats) {
    return null
  }

  const statCards = [
    {
      title: 'Total Free Users',
      value: stats.totalFreeUsers.toLocaleString(),
      icon: Users,
      description: 'All current free users',
      color: 'text-blue-600',
      bgColor: 'bg-blue-50 dark:bg-blue-950',
    },
    {
      title: 'Active Free Users',
      value: stats.activeFreeUsers.toLocaleString(),
      icon: UserCheck,
      description: 'Users active in last 7 days',
      color: 'text-green-600',
      bgColor: 'bg-green-50 dark:bg-green-950',
    },
    {
      title: 'Total Registration Bonus',
      value: stats.totalRegistrationBonusGranted.toLocaleString(),
      icon: Gift,
      description: 'Total registration bonus granted',
      color: 'text-purple-600',
      bgColor: 'bg-purple-50 dark:bg-purple-950',
    },
    {
      title: 'Total Periodic Points Granted',
      value: stats.totalPeriodicPointsGranted.toLocaleString(),
      icon: Calendar,
      description: 'Total periodic points granted',
      color: 'text-orange-600',
      bgColor: 'bg-orange-50 dark:bg-orange-950',
    },
  ]

  return (
    <div className="space-y-6" data-testid="free-stats-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h1 id="page-title" className="text-xl font-semibold">
          免费用户统计
        </h1>
        <div className="flex gap-2" role="group" aria-label="操作按钮">
          <Button
            variant="outline"
            size="sm"
            onClick={() => refetch()}
            data-testid="refresh-button"
            aria-label="Refresh stats"
            disabled={isLoading || isRefetching}
          >
            <RefreshCw
              className={`h-4 w-4 mr-2 ${isRefetching ? 'animate-spin' : ''}`}
              aria-hidden="true"
            />
            {isRefetching ? 'Refreshing...' : 'Refresh'}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleExport}
            data-testid="export-button"
            aria-label="Export CSV"
            disabled={!stats}
          >
            <Download className="h-4 w-4 mr-2" aria-hidden="true" />
            Export CSV
          </Button>
        </div>
      </div>

      {/* Date Range Filter */}
      <Card aria-labelledby="date-filter-title">
        <CardHeader>
          <CardTitle id="date-filter-title">Date Range Filter</CardTitle>
          <CardDescription>Optional: filter stats by time period</CardDescription>
        </CardHeader>
        <CardContent>
          <div
            className="grid grid-cols-1 md:grid-cols-2 gap-4"
            role="group"
            aria-label="Date filter"
          >
            <div className="space-y-2">
              <Label htmlFor="start-date" id="start-date-label">
                Start Date
              </Label>
              <Input
                id="start-date"
                type="date"
                value={startDate || ''}
                onChange={(e) => handleDateChange('startDate', e.target.value)}
                data-testid="start-date-input"
                aria-labelledby="start-date-label"
                aria-describedby="date-filter-hint"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="end-date" id="end-date-label">
                End Date
              </Label>
              <Input
                id="end-date"
                type="date"
                value={endDate || ''}
                onChange={(e) => handleDateChange('endDate', e.target.value)}
                data-testid="end-date-input"
                aria-labelledby="end-date-label"
                aria-describedby="date-filter-hint"
              />
            </div>
          </div>
          <p id="date-filter-hint" className="text-xs text-muted-foreground mt-2">
            Leave empty for no date restriction
          </p>
        </CardContent>
      </Card>

      {/* Statistics Cards */}
      <div
        className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4"
        role="region"
        aria-label="Statistics cards"
        aria-live="polite"
      >
        {statCards.map((card) => (
          <Card
            key={card.title}
            data-testid={`${card.title.toLowerCase().replace(/\s+/g, '-')}-card`}
            aria-labelledby={`${card.title.toLowerCase().replace(/\s+/g, '-')}-title`}
            aria-describedby={`${card.title.toLowerCase().replace(/\s+/g, '-')}-description`}
          >
            <CardContent className="pt-6">
              <div className="flex items-center justify-between">
                <div className="space-y-2">
                  <p
                    id={`${card.title.toLowerCase().replace(/\s+/g, '-')}-title`}
                    className="text-sm font-medium text-muted-foreground"
                  >
                    {card.title}
                  </p>
                  <p className="text-2xl font-bold" aria-label={`${card.title}: ${card.value}`}>
                    {card.value}
                  </p>
                  <p
                    id={`${card.title.toLowerCase().replace(/\s+/g, '-')}-description`}
                    className="text-xs text-muted-foreground"
                  >
                    {card.description}
                  </p>
                </div>
                <div className={`p-3 rounded-lg ${card.bgColor}`} aria-hidden="true">
                  <card.icon className={`h-6 w-6 ${card.color}`} />
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      <div
        className="grid grid-cols-1 md:grid-cols-2 gap-4"
        role="region"
        aria-label="Detailed metrics"
      >
        <Card
          data-testid="average-periodic-points-card"
          aria-labelledby="average-periodic-points-title"
          aria-describedby="average-periodic-points-description"
        >
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div className="space-y-2">
                <p
                  id="average-periodic-points-title"
                  className="text-sm font-medium text-muted-foreground"
                >
                  Avg Periodic Points/User
                </p>
                <p
                  className="text-2xl font-bold"
                  aria-label={`Avg Periodic Points/User: ${stats.averagePeriodicPointsPerUser.toFixed(2)}`}
                >
                  {stats.averagePeriodicPointsPerUser.toFixed(2)}
                </p>
                <p
                  id="average-periodic-points-description"
                  className="text-xs text-muted-foreground"
                >
                  Average periodic points per user
                </p>
              </div>
              <div className="p-3 rounded-lg bg-cyan-50 dark:bg-cyan-950" aria-hidden="true">
                <TrendingUp className="h-6 w-6 text-cyan-600" />
              </div>
            </div>
          </CardContent>
        </Card>

        <Card
          data-testid="upgrade-rate-card"
          aria-labelledby="upgrade-rate-title"
          aria-describedby="upgrade-rate-description"
        >
          <CardContent className="pt-6">
            <div className="flex items-center justify-between">
              <div className="space-y-2">
                <p id="upgrade-rate-title" className="text-sm font-medium text-muted-foreground">
                  Paid Conversion Rate
                </p>
                <p
                  className="text-2xl font-bold"
                  aria-label={`Paid Conversion Rate: ${stats.upgradeRate.toFixed(2)}%`}
                >
                  {stats.upgradeRate.toFixed(2)}%
                </p>
                <p id="upgrade-rate-description" className="text-xs text-muted-foreground">
                  Free-to-paid user conversion rate
                </p>
              </div>
              <div
                className={`p-3 rounded-lg ${
                  stats.upgradeRate > 10
                    ? 'bg-green-50 dark:bg-green-950'
                    : stats.upgradeRate > 5
                      ? 'bg-yellow-50 dark:bg-yellow-950'
                      : 'bg-red-50 dark:bg-red-950'
                }`}
                aria-hidden="true"
              >
                <TrendingUp
                  className={`h-6 w-6 ${
                    stats.upgradeRate > 10
                      ? 'text-green-600'
                      : stats.upgradeRate > 5
                        ? 'text-yellow-600'
                        : 'text-red-600'
                  }`}
                />
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Last Updated */}
      <Alert role="status" aria-live="polite">
        <AlertDescription className="text-xs text-muted-foreground flex items-center justify-between">
          <span>
            Last updated: {format(new Date(stats.lastUpdatedAt), 'yyyy-MM-dd HH:mm:ss')}(
            {relativeTime})
          </span>
          <span className="flex items-center gap-1">
            {isRefetching ? (
              <>
                <Loader2 className="h-3 w-3 animate-spin" aria-hidden="true" />
                Refreshing...
              </>
            ) : (
              <>
                <RefreshCw className="h-3 w-3" aria-hidden="true" />
                Auto-refreshes every 5 minutes
              </>
            )}
          </span>
        </AlertDescription>
      </Alert>
    </div>
  )
}
