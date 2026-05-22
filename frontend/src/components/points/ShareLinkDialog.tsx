import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Copy, Check } from 'lucide-react'
import { useCopyToClipboard } from '@/hooks/use-copy-to-clipboard'

interface ShareLinkDialogProps {
  open: boolean
  onClose: () => void
  guideUrl: string
}

export function ShareLinkDialog({ open, onClose, guideUrl }: ShareLinkDialogProps) {
  const { copied, copyToClipboard } = useCopyToClipboard()

  async function handleCopy() {
    await copyToClipboard(guideUrl)
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="max-w-md" data-testid="share-link-dialog">
        <DialogHeader>
          <DialogTitle>Share Guide Link</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="text-sm text-muted-foreground">
            Share this link with users to show them points recharge rules for this plan.
          </div>
          <div className="flex gap-2">
            <Input value={guideUrl} readOnly className="flex-1" />
            <Button onClick={handleCopy} variant="outline" size="icon">
              {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}
