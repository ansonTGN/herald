import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { m } from '@/paraglide/messages'

/**
 * Protected-price confirmation dialog.
 *
 * Used ONLY for the protected-price second-confirm that appears when a batch
 * save returns 409 `mapping_in_use` after the admin toggles `enabled` off on a
 * price that protects active subscriptions.
 *
 * The dialog is purely informational + acknowledge: the backend rejects the
 * disable (the batch rolled back), so there is no "force" action here — the
 * admin closes it and adjusts their edits. The active-subscription count is
 * surfaced for transparency.
 */
interface ProtectedPriceConfirmDialogProps {
  open: boolean
  activeSubscriptions: number | null
  onOpenChange: (open: boolean) => void
}

export function ProtectedPriceConfirmDialog({
  open,
  activeSubscriptions,
  onOpenChange,
}: ProtectedPriceConfirmDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md" data-testid="protected-price-confirm-dialog">
        <DialogHeader>
          <DialogTitle>{m['billing.protected_price_confirm_title']()}</DialogTitle>
          <DialogDescription>
            {m['billing.protected_price_confirm_body']({
              count: activeSubscriptions ?? 0,
            })}
          </DialogDescription>
        </DialogHeader>
        {activeSubscriptions != null && (
          <p
            className="text-sm font-medium text-destructive"
            data-testid="protected-price-active-subs"
          >
            {m['billing.protected_price_active_subs']({ count: activeSubscriptions })}
          </p>
        )}
        <DialogFooter showCloseButton>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="protected-price-confirm-cancel"
          >
            {m['billing.protected_price_confirm_cancel']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
