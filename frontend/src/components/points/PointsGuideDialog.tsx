import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Badge } from '@/components/ui/badge'
import { Calendar, Info } from 'lucide-react'
import type { PointsPlanConfigResponse } from '@/lib/api-generated'

interface PointsGuideDialogProps {
  config: PointsPlanConfigResponse | null
  planName: string
  open: boolean
  onClose: () => void
}

export function PointsGuideDialog({ config, planName, open, onClose }: PointsGuideDialogProps) {
  if (!config) return null

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-md" data-testid="points-guide-dialog">
        <DialogHeader>
          <DialogTitle>Points Recharge Guide</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div>
            <div className="text-sm text-muted-foreground mb-2">Plan</div>
            <div className="font-semibold text-lg">{planName}</div>
          </div>

          <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
            <div className="flex items-center gap-2">
              <Info className="h-4 w-4" />
              <span className="text-sm">Points per Period</span>
            </div>
            <Badge variant="default" className="text-lg px-3 py-1">
              +{config.pointsPerPeriod.toLocaleString()}
            </Badge>
          </div>

          <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
            <div className="flex items-center gap-2">
              <Calendar className="h-4 w-4" />
              <span className="text-sm">Grant Period ({config.grantPeriodType})</span>
            </div>
            <Badge variant="outline" className="text-lg px-3 py-1">
              {config.grantPeriodType}
            </Badge>
          </div>

          {config.grantOnSubscribe && (
            <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
              <div className="flex items-center gap-2">
                <Info className="h-4 w-4" />
                <span className="text-sm">Grant on Subscribe</span>
              </div>
              <Badge variant="default" className="text-lg px-3 py-1">
                Yes
              </Badge>
            </div>
          )}

          <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
            <div className="flex items-center gap-2">
              <Calendar className="h-4 w-4" />
              <span className="text-sm">Validity Days</span>
            </div>
            <Badge variant="outline" className="text-lg px-3 py-1">
              {config.validityDays} days
            </Badge>
          </div>

          {config.maxPeriods && (
            <div className="text-sm text-muted-foreground">
              Maximum periods: {config.maxPeriods.toLocaleString()}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
