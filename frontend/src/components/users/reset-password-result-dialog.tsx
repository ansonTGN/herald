import { toast } from 'sonner'
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { useCopyToClipboard } from '@/hooks/use-copy-to-clipboard'

interface ResetPasswordResultDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  newPassword: string
}

export function ResetPasswordResultDialog({
  open,
  onOpenChange,
  newPassword,
}: ResetPasswordResultDialogProps) {
  const { copied, copyToClipboard } = useCopyToClipboard()

  async function handleCopy() {
    try {
      await copyToClipboard(newPassword)
    } catch {
      toast.error('Failed to copy password to clipboard')
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-testid="reset-password-result-dialog">
        <DialogHeader>
          <DialogTitle>Password Reset Successful</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="rounded-md border bg-muted p-3">
            <code className="block break-all text-sm font-mono" data-testid="new-password-text">
              {newPassword}
            </code>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleCopy}
            data-testid="copy-password-button"
          >
            {copied ? 'Copied!' : 'Copy Password'}
          </Button>
          <p className="text-sm text-muted-foreground">
            Please securely share this password with the user.
          </p>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Close
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
