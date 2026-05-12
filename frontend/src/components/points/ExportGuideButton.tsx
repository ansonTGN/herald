import { Button } from '@/components/ui/button'
import { Download } from 'lucide-react'

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
      'Plan',
      'Points per Period',
      'Grant on Subscribe',
      'Grant Period',
      'Validity Days',
      'Max Periods',
    ]
    const rows = configs.map((c) => [
      c.planName,
      c.pointsPerPeriod.toString(),
      c.grantOnSubscribe ? 'Yes' : 'No',
      c.grantPeriodType,
      c.validityDays.toString(),
      c.maxPeriods?.toString() || 'Unlimited',
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
      Export Guide
    </Button>
  )
}
