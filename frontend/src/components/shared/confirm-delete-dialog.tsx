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

interface ConfirmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description: React.ReactNode
  onConfirm: () => void
  confirmLabel?: string
  cancelLabel?: string
  isPending?: boolean
  confirmDisabled?: boolean
  confirmClassName?: string
  contentTestId?: string
  cancelTestId?: string
  confirmTestId?: string
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  onConfirm,
  confirmLabel = 'Delete',
  cancelLabel = 'Cancel',
  isPending = false,
  confirmDisabled = false,
  confirmClassName,
  contentTestId,
  cancelTestId,
  confirmTestId,
}: ConfirmDialogProps) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent {...(contentTestId && { 'data-testid': contentTestId })}>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel
            disabled={isPending}
            {...(cancelTestId && { 'data-testid': cancelTestId })}
          >
            {cancelLabel}
          </AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            disabled={confirmDisabled || isPending}
            className={
              confirmClassName ??
              'bg-destructive text-destructive-foreground hover:bg-destructive/90'
            }
            {...(confirmTestId && { 'data-testid': confirmTestId })}
          >
            {isPending ? `${confirmLabel}...` : confirmLabel}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
