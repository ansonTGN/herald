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

interface RegenerateSecretDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
  clientAppName: string
}

export function RegenerateSecretDialog({
  open,
  onOpenChange,
  onConfirm,
  clientAppName,
}: RegenerateSecretDialogProps) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent data-testid="regenerate-secret-dialog">
        <AlertDialogHeader>
          <AlertDialogTitle>Regenerate Client Secret?</AlertDialogTitle>
          <AlertDialogDescription>
            This will invalidate the current client secret for <strong>{clientAppName}</strong>. You
            will need to update all applications using this client app with the new secret.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel data-testid="cancel-regenerate-button">Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            className="bg-red-600 hover:bg-red-700"
            data-testid="confirm-regenerate-button"
          >
            Regenerate Secret
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
