import { Button } from '@/components/ui/button'
import { Download } from 'lucide-react'
import { m } from '@/paraglide/messages'

interface ExportGuideButtonProps {
  configs: Array<{
    planName: string
    pointsPerPeriod: number
    grantOnSubscribe: boolean
    grantPeriodType: string
    maxPeriods: number | null
    validityDays: number
  }>
}

export function ExportGuideButton({ configs }: ExportGuideButtonProps) {
  function handleExport() {
    const headers = [
      m['points.export_csv_headers_plan'](),
      m['points.export_csv_headers_points'](),
      m['points.export_csv_headers_grant_subscribe'](),
      m['points.export_csv_headers_period'](),
      m['points.export_csv_headers_validity'](),
      m['points.export_csv_headers_max_periods'](),
    ]
    const rows = configs.map((c) => [
      c.planName,
      c.pointsPerPeriod.toString(),
      c.grantOnSubscribe ? m['common.yes']() : m['common.no'](),
      c.grantPeriodType,
      c.validityDays.toString(),
      c.maxPeriods?.toString() || m['points.export_csv_unlimited'](),
    ])

    const csv = [headers, ...rows].map((row) => row.map((cell) => `"${cell}"`).join(',')).join('\n')

    const blob = new Blob([csv], { type: 'text/csv' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'points-recharge-guide.csv'
    a.click()
    URL.revokeObjectURL(url)
  }

  return (
    <Button variant="outline" onClick={handleExport} data-testid="export-guide-button">
      <Download className="mr-2 h-4 w-4" />
      {m['points.configs_export_guide']()}
    </Button>
  )
}
