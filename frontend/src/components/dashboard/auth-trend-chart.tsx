import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
  type ChartConfig,
} from '@/components/ui/chart'
import { m } from '@/paraglide/messages'

interface AuthTrendChartProps {
  data: Array<{
    date: string
    successCount: number
    failureCount: number
  }>
  testId?: string
}

function formatShortDate(dateStr: string): string {
  const date = new Date(dateStr + 'T00:00:00')
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })
}

export function AuthTrendChart({ data, testId }: AuthTrendChartProps) {
  const chartConfig = {
    successCount: {
      label: m['dashboard.login_success'](),
      color: 'var(--chart-1)',
    },
    failureCount: {
      label: m['dashboard.login_failed'](),
      color: 'var(--destructive)',
    },
  } satisfies ChartConfig

  return (
    <Card data-testid={testId}>
      <CardHeader>
        <CardTitle>{m['dashboard.login_activity']()}</CardTitle>
        <CardDescription>{m['dashboard.last_30_days']()}</CardDescription>
      </CardHeader>
      <CardContent>
        {data.length === 0 ? (
          <div className="flex h-[250px] items-center justify-center text-muted-foreground">
            {m['dashboard.no_data_available']()}
          </div>
        ) : (
          <ChartContainer config={chartConfig} className="h-[250px] w-full">
            <AreaChart data={data} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
              <CartesianGrid vertical={false} />
              <XAxis
                dataKey="date"
                tickLine={false}
                axisLine={false}
                tickMargin={8}
                tickFormatter={formatShortDate}
              />
              <YAxis tickLine={false} axisLine={false} tickMargin={8} />
              <ChartTooltip
                content={
                  <ChartTooltipContent
                    labelFormatter={(_, payload) => {
                      if (payload?.[0]?.payload?.date) {
                        return formatShortDate(payload[0].payload.date)
                      }
                      return ''
                    }}
                  />
                }
              />
              <ChartLegend content={<ChartLegendContent />} />
              <Area
                type="monotone"
                dataKey="successCount"
                stroke="var(--color-successCount)"
                fill="var(--color-successCount)"
                fillOpacity={0.2}
                strokeWidth={2}
              />
              <Area
                type="monotone"
                dataKey="failureCount"
                stroke="var(--color-failureCount)"
                fill="var(--color-failureCount)"
                fillOpacity={0.2}
                strokeWidth={2}
                strokeDasharray="5 5"
              />
            </AreaChart>
          </ChartContainer>
        )}
      </CardContent>
    </Card>
  )
}
