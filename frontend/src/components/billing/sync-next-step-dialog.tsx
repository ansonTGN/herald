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
 * Post-sync "next step" guidance dialog.
 *
 * Synced entitlement mappings are created as disabled drafts by the backend.
 * After a successful sync, if EVERY currently-loaded mapping is still
 * disabled, this dialog surfaces once to tell the admin what to do next
 * (configure entitlement key / points quota per price, then enable + save).
 *
 * Pure informational + acknowledge: a single "Got it" button closes it. The
 * trigger condition is gated by a one-shot flag in the page so it only
 * appears right after a sync, never on plain page visits.
 */
interface SyncNextStepDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SyncNextStepDialog({ open, onOpenChange }: SyncNextStepDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md" data-testid="sync-next-step-dialog">
        <DialogHeader>
          <DialogTitle>{m['billing.sync_next_step_title']()}</DialogTitle>
          <DialogDescription>{m['billing.sync_next_step_body']()}</DialogDescription>
        </DialogHeader>
        <DialogFooter showCloseButton>
          <Button onClick={() => onOpenChange(false)} data-testid="sync-next-step-dismiss">
            {m['billing.sync_next_step_dismiss']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
