import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { AlertTriangle } from 'lucide-react'
import type { PointsPackageResponse } from '@/lib/api-generated'
import { formatPrice } from '@/lib/schemas/points-package-forms'
import { m } from '@/paraglide/messages'

interface PointsPackageDeleteDialogProps {
  package: PointsPackageResponse
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
  isDeleting: boolean
  hasPurchaseHistory: boolean
}

export function PointsPackageDeleteDialog({
  package: pkg,
  open,
  onOpenChange,
  onConfirm,
  isDeleting,
  hasPurchaseHistory,
}: PointsPackageDeleteDialogProps) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent data-testid="points-package-delete-dialog">
        <AlertDialogHeader>
          <div className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-destructive" />
            <AlertDialogTitle>{m['points.packages_delete_dialog_title']()}</AlertDialogTitle>
          </div>
          <AlertDialogDescription>
            {hasPurchaseHistory ? (
              <div className="space-y-2">
                <p>{m['points.packages_delete_dialog_has_history']()}</p>
                <div
                  className="rounded-md bg-destructive/10 p-3 text-sm text-destructive"
                  data-testid="points-package-delete-warning-message"
                >
                  <strong>{m['points.packages_delete_dialog_has_history_warning']()}</strong>
                </div>
              </div>
            ) : (
              <div className="space-y-2">
                <p>{m['points.packages_delete_dialog_confirm']({ title: pkg.title })}</p>
                <p className="text-sm text-muted-foreground">
                  {m['points.packages_delete_dialog_confirm_detail']({
                    points: pkg.points.toLocaleString(),
                    price: formatPrice(pkg.price, pkg.currency),
                  })}
                </p>
                <p className="text-sm text-destructive font-medium">
                  {m['points.packages_delete_dialog_cannot_undo']()}
                </p>
              </div>
            )}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel
            onClick={() => onOpenChange(false)}
            data-testid="points-package-delete-cancel-button"
          >
            {m['common.cancel']()}
          </AlertDialogCancel>
          {!hasPurchaseHistory && (
            <AlertDialogAction
              onClick={(e) => {
                e.preventDefault()
                onConfirm()
              }}
              disabled={isDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              data-testid="points-package-delete-confirm-button"
            >
              {isDeleting
                ? m['points.packages_delete_dialog_deleting']()
                : m['points.packages_delete_dialog_button']()}
            </AlertDialogAction>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
