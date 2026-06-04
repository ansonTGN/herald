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
import { m } from '@/paraglide/messages'

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
          <AlertDialogTitle>{m['billing.delete_provider_title']({ configType })}</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-2">
              {!canDelete ? (
                <>
                  <p>{m['billing.delete_provider_active_subs']({ count: activeSubscriptions })}</p>
                  <p>{m['billing.delete_provider_migrate']()}</p>
                </>
              ) : (
                <>
                  <p>{m['billing.delete_provider_confirm']({ configType })}</p>
                  <p className="text-destructive font-medium">
                    {m['billing.delete_provider_warning']()}
                  </p>
                </>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>

        {!canDelete && (
          <Alert className="border-destructive bg-destructive/10">
            <AlertDescription>
              {m['billing.active_subscriptions']({ count: activeSubscriptions })}
            </AlertDescription>
          </Alert>
        )}

        <AlertDialogFooter>
          <AlertDialogCancel data-testid="delete-cancel-button" disabled={isDeleting}>
            {m['common.cancel']()}
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
              {isDeleting ? m['billing.deleting']() : m['billing.delete_configuration']()}
            </AlertDialogAction>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
