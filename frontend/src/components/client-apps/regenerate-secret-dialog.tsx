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
import { m } from '@/paraglide/messages'

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
          <AlertDialogTitle>{m['client_apps.regenerate_title']()}</AlertDialogTitle>
          <AlertDialogDescription>
            {m['client_apps.regenerate_description']({ name: clientAppName })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel data-testid="cancel-regenerate-button">
            {m['common.cancel']()}
          </AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            className="bg-destructive hover:bg-destructive/90"
            data-testid="confirm-regenerate-button"
          >
            {m['client_apps.regenerate_button']()}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
