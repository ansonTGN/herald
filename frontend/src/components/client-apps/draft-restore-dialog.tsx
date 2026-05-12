import { useState, useEffect } from 'react'
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { formatDraftAge, type DraftData } from '@/hooks/use-draft-autosave'

export interface DraftRestoreDialogProps<T> {
  /** Whether the dialog is open */
  open: boolean
  /** Draft data to restore */
  draft: DraftData<T>
  /** Callback when user confirms restore */
  onRestore: () => void
  /** Callback when user discards draft */
  onDiscard: () => void
  /** Callback when dialog is closed without action */
  onClose?: () => void
}

/**
 * Dialog component for restoring auto-saved drafts
 *
 * Shows draft age and provides options to restore or discard the draft
 */
export function DraftRestoreDialog<T>({
  open,
  draft,
  onRestore,
  onDiscard,
  onClose,
}: DraftRestoreDialogProps<T>) {
  // Calculate draft age on render - it's a pure function of draft.timestamp
  const draftAge = formatDraftAge(draft.timestamp)

  // Dummy state to force re-renders
  const [, setForceUpdate] = useState(0)

  // Update the component every minute to refresh the age display
  useEffect(() => {
    if (!open) return

    // Force a re-render every minute by updating state
    const interval = setInterval(() => {
      setForceUpdate((prev) => prev + 1)
    }, 60000)

    return () => clearInterval(interval)
  }, [open])

  const handleRestore = () => {
    onRestore()
  }

  const handleDiscard = () => {
    onDiscard()
  }

  const handleClose = () => {
    onClose?.()
  }

  return (
    <AlertDialog open={open} onOpenChange={(open) => !open && handleClose()}>
      <AlertDialogContent data-testid="draft-restore-dialog">
        <AlertDialogHeader>
          <AlertDialogTitle data-testid="draft-restore-dialog-title">
            Restore Draft?
          </AlertDialogTitle>
          <AlertDialogDescription data-testid="draft-restore-dialog-description">
            We found an auto-saved draft from <span className="font-semibold">{draftAge}</span>.
            Would you like to restore it or start fresh?
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="py-4">
          <div className="rounded-md bg-muted p-4">
            <p className="text-sm text-muted-foreground">
              <span className="font-medium">Draft Details:</span>
            </p>
            <ul className="mt-2 space-y-1 text-sm text-muted-foreground">
              <li>• Saved: {new Date(draft.timestamp).toLocaleString()}</li>
              <li>• Version: {draft.version}</li>
            </ul>
          </div>
        </div>

        <AlertDialogFooter>
          <Button variant="outline" onClick={handleDiscard} data-testid="draft-discard-button">
            Discard Draft
          </Button>
          <Button onClick={handleRestore} data-testid="draft-restore-button">
            Restore Draft
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}

export interface MultipleDraftsDialogProps {
  /** Whether the dialog is open */
  open: boolean
  /** List of available drafts */
  drafts: Array<{
    draftKey: string
    timestamp: number
    version: string
  }>
  /** Callback when user selects a draft to restore */
  onRestore: (draftKey: string) => void
  /** Callback when user discards all drafts */
  onDiscardAll: () => void
  /** Callback when dialog is closed without action */
  onClose?: () => void
}

/**
 * Dialog for handling multiple drafts
 * Allows user to select which draft to restore
 */
export function MultipleDraftsDialog({
  open,
  drafts,
  onRestore,
  onDiscardAll,
  onClose,
}: MultipleDraftsDialogProps) {
  const handleClose = () => {
    onClose?.()
  }

  return (
    <AlertDialog open={open} onOpenChange={(open) => !open && handleClose()}>
      <AlertDialogContent data-testid="multiple-drafts-dialog" className="max-w-2xl">
        <AlertDialogHeader>
          <AlertDialogTitle data-testid="multiple-drafts-dialog-title">
            Multiple Drafts Found
          </AlertDialogTitle>
          <AlertDialogDescription data-testid="multiple-drafts-dialog-description">
            We found multiple auto-saved drafts. Select one to restore or discard all.
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="py-4 max-h-[400px] overflow-y-auto">
          <div className="space-y-2">
            {drafts.map((draft) => (
              <div
                key={draft.draftKey}
                className="flex items-center justify-between rounded-md border p-4"
              >
                <div>
                  <p className="font-medium">{draft.draftKey}</p>
                  <p className="text-sm text-muted-foreground">{formatDraftAge(draft.timestamp)}</p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => onRestore(draft.draftKey)}
                  data-testid={`restore-draft-${draft.draftKey}`}
                >
                  Restore
                </Button>
              </div>
            ))}
          </div>
        </div>

        <AlertDialogFooter>
          <Button
            variant="outline"
            onClick={handleClose}
            data-testid="multiple-drafts-cancel-button"
          >
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={onDiscardAll}
            data-testid="discard-all-drafts-button"
          >
            Discard All
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
