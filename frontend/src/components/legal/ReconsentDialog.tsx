import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { AgreementLinks } from '@/components/legal/AgreementLinks'
import { m } from '@/paraglide/messages'
import type { ConsentStatusItem } from '@/lib/api-generated'

interface ReconsentDialogProps {
  realmId: string
  open: boolean
  items: ConsentStatusItem[]
  isPending: boolean
  onAgree: () => void
  onLogout: () => void
}

export function ReconsentDialog({
  realmId,
  open,
  items,
  isPending,
  onAgree,
  onLogout,
}: ReconsentDialogProps) {
  const pendingItems = items.filter((item) => item.needs_reconsent)

  return (
    <Dialog open={open} onOpenChange={() => {}}>
      <DialogContent showCloseButton={false} className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle data-testid="reconsent-dialog-title">{m['reconsent.title']()}</DialogTitle>
          <DialogDescription data-testid="reconsent-dialog-description">
            {m['reconsent.description']()}
          </DialogDescription>
        </DialogHeader>
        <div className="space-y-3 py-2">
          {pendingItems.map((item) => (
            <div
              key={item.agreement_type}
              className="rounded border p-3"
              data-testid={`reconsent-agreement-${item.agreement_type}`}
            >
              <div className="font-medium">
                <AgreementLinks
                  realmId={realmId}
                  agreementType={item.agreement_type as 'terms_of_service' | 'privacy_policy'}
                />
              </div>
            </div>
          ))}
        </div>
        <DialogFooter className="flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Button
            type="button"
            variant="outline"
            onClick={onLogout}
            disabled={isPending}
            data-testid="reconsent-logout-button"
          >
            {m['reconsent.logout_button']()}
          </Button>
          <Button
            type="button"
            onClick={onAgree}
            disabled={isPending}
            data-testid="reconsent-agree-button"
          >
            {isPending ? m['common.loading']() : m['reconsent.agree_button']()}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
