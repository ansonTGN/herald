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
import { Alert, AlertDescription } from '@/components/ui/alert'

interface DeleteConfirmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
  configType: string
  activeSubscriptions?: number
  isDeleting?: boolean
}

export function DeleteConfirmDialog({
  open,
  onOpenChange,
  onConfirm,
  configType,
  activeSubscriptions = 0,
  isDeleting = false,
}: DeleteConfirmDialogProps) {
  const canDelete = activeSubscriptions === 0

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent data-testid="delete-confirm-dialog">
        <AlertDialogHeader>
          <AlertDialogTitle>Delete {configType} Configuration?</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-2">
              {!canDelete ? (
                <>
                  <p>
                    This configuration cannot be deleted because there are {activeSubscriptions}{' '}
                    active subscription(s) using it.
                  </p>
                  <p>
                    Please cancel or migrate all active subscriptions before deleting this
                    configuration.
                  </p>
                </>
              ) : (
                <>
                  <p>Are you sure you want to delete this {configType} configuration?</p>
                  <p className="text-destructive font-medium">
                    This action cannot be undone. All webhook endpoints will stop working.
                  </p>
                </>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>

        {!canDelete && (
          <Alert className="border-destructive bg-destructive/10">
            <AlertDescription>Active subscriptions: {activeSubscriptions}</AlertDescription>
          </Alert>
        )}

        <AlertDialogFooter>
          <AlertDialogCancel data-testid="delete-cancel-button" disabled={isDeleting}>
            Cancel
          </AlertDialogCancel>
          {canDelete && (
            <AlertDialogAction
              onClick={(e) => {
                e.preventDefault()
                onConfirm()
              }}
              disabled={isDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              data-testid="delete-confirm-button"
            >
              {isDeleting ? 'Deleting...' : 'Delete Configuration'}
            </AlertDialogAction>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
