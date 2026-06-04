import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Badge } from '@/components/ui/badge'
import { Calendar, Info } from 'lucide-react'
import type { PointsPlanConfigResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

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
          <DialogTitle>{m['points.guide_dialog_title']()}</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div>
            <div className="text-sm text-muted-foreground mb-2">
              {m['points.guide_dialog_plan']()}
            </div>
            <div className="font-semibold text-lg">{planName}</div>
          </div>

          <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
            <div className="flex items-center gap-2">
              <Info className="h-4 w-4" />
              <span className="text-sm">{m['points.guide_dialog_points_per_period']()}</span>
            </div>
            <Badge variant="default" className="text-lg px-3 py-1">
              +{config.pointsPerPeriod.toLocaleString()}
            </Badge>
          </div>

          <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
            <div className="flex items-center gap-2">
              <Calendar className="h-4 w-4" />
              <span className="text-sm">
                {m['points.guide_dialog_grant_period']({ type: config.grantPeriodType })}
              </span>
            </div>
            <Badge variant="outline" className="text-lg px-3 py-1">
              {config.grantPeriodType}
            </Badge>
          </div>

          {config.grantOnSubscribe && (
            <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
              <div className="flex items-center gap-2">
                <Info className="h-4 w-4" />
                <span className="text-sm">{m['points.guide_dialog_grant_on_subscribe']()}</span>
              </div>
              <Badge variant="default" className="text-lg px-3 py-1">
                {m['common.yes']()}
              </Badge>
            </div>
          )}

          <div className="flex items-center justify-between p-3 bg-muted rounded-lg">
            <div className="flex items-center gap-2">
              <Calendar className="h-4 w-4" />
              <span className="text-sm">{m['points.guide_dialog_validity_days']()}</span>
            </div>
            <Badge variant="outline" className="text-lg px-3 py-1">
              {m['points.guide_dialog_validity_days_value']({ days: config.validityDays })}
            </Badge>
          </div>

          {config.maxPeriods && (
            <div className="text-sm text-muted-foreground">
              {m['points.guide_dialog_max_periods']({ count: config.maxPeriods.toLocaleString() })}
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
