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

/**
 * Shape of a 409 `bucket_in_use` error body (design §4.2.3 / generated
 * `BucketInUseErrorBody`). Surfaced by `useDeleteCreditBucket` when the
 * backend refuses deletion due to in-flight subscriptions or residual
 * balances.
 */
export interface BucketInUseError {
  code?: string
  activeSubscriptions?: number
  holdersWithBalance?: number
}

interface DeleteBucketConfirmDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  onConfirm: () => void
  bucketName: string
  /** Populated when the delete attempt returned 409 `bucket_in_use`. */
  inUseError?: BucketInUseError | null
  isDeleting?: boolean
}

/**
 * Destructive-confirm AlertDialog for deleting a Credit Bucket.
 *
 * - When `inUseError` is null/absent → normal confirm (name + destructive action).
 * - When `inUseError` is present (409 `bucket_in_use`) → the confirm Action is
 *   hidden and a danger alert shows `activeSubscriptions` / `holdersWithBalance`
 *   so the admin understands why deletion was refused. This mirrors the
 *   `components/billing/DeleteConfirmDialog.tsx` "canDelete" gating pattern.
 */
export function DeleteBucketConfirmDialog({
  open,
  onOpenChange,
  onConfirm,
  bucketName,
  inUseError,
  isDeleting = false,
}: DeleteBucketConfirmDialogProps) {
  const blocked = !!inUseError

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent data-testid="delete-bucket-confirm-dialog">
        <AlertDialogHeader>
          <AlertDialogTitle>{m['credit_buckets.delete_title']()}</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-2">
              {blocked ? (
                <p>{m['credit_buckets.delete_in_use']({ name: bucketName })}</p>
              ) : (
                <>
                  <p>{m['credit_buckets.delete_confirm']({ name: bucketName })}</p>
                  <p className="text-destructive font-medium">
                    {m['credit_buckets.delete_warning']()}
                  </p>
                </>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>

        {blocked && (
          <Alert variant="destructive" data-testid="delete-bucket-error-message">
            <AlertDescription>
              {m['credit_buckets.delete_in_use_detail']({
                activeSubscriptions: inUseError?.activeSubscriptions ?? 0,
                holdersWithBalance: inUseError?.holdersWithBalance ?? 0,
              })}
            </AlertDescription>
          </Alert>
        )}

        <AlertDialogFooter>
          <AlertDialogCancel data-testid="delete-bucket-cancel-button" disabled={isDeleting}>
            {m['common.cancel']()}
          </AlertDialogCancel>
          {!blocked && (
            <AlertDialogAction
              onClick={(e) => {
                e.preventDefault()
                onConfirm()
              }}
              disabled={isDeleting}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              data-testid="delete-bucket-confirm-button"
            >
              {isDeleting ? m['credit_buckets.deleting']() : m['credit_buckets.delete_button']()}
            </AlertDialogAction>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
