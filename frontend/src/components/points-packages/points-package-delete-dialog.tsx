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
    <AlertDialog open={open} onOpenChange={onOpenChange} data-testid="points-package-delete-dialog">
      <AlertDialogContent>
        <AlertDialogHeader>
          <div className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-destructive" />
            <AlertDialogTitle>Delete Points Package?</AlertDialogTitle>
          </div>
          <AlertDialogDescription>
            {hasPurchaseHistory ? (
              <div className="space-y-2">
                <p>
                  This package has purchase history and cannot be deleted. You can only disable it
                  to prevent future purchases.
                </p>
                <div
                  className="rounded-md bg-destructive/10 p-3 text-sm text-destructive"
                  data-testid="points-package-delete-warning-message"
                >
                  <strong>Warning:</strong> Packages with existing purchases cannot be deleted for
                  data integrity reasons.
                </div>
              </div>
            ) : (
              <div className="space-y-2">
                <p>
                  Are you sure you want to delete the points package <strong>"{pkg.title}"</strong>?
                </p>
                <p className="text-sm text-muted-foreground">
                  This will permanently delete the package containing{' '}
                  <strong>{pkg.points.toLocaleString()}</strong> points priced at{' '}
                  {formatPrice(pkg.price, pkg.currency)}.
                </p>
                <p className="text-sm text-destructive font-medium">
                  This action cannot be undone.
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
            Cancel
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
              {isDeleting ? 'Deleting...' : 'Delete Package'}
            </AlertDialogAction>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
