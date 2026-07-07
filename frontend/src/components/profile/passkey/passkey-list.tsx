import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { useAppForm, AppForm } from '@/components/ui/tanstack-form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent } from '@/components/ui/card'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { withTimeout } from '@/lib/totp-utils'
import { formatDateTimeShort } from '@/lib/date-utils'
import { passkeyListQueryOptions, queryKeys } from '@/data/query-options'
import { handleRenamePasskeyCredential, handleDeletePasskeyCredential } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { z } from 'zod'
import { m } from '@/paraglide/messages'
import type { PasskeyCredentialViewResponse } from '@/lib/api-generated'

const nicknameSchema = z.object({
  nickname: z.string().min(1).max(128),
})

interface PasskeyListProps {
  /**
   * Called when the user clicks the "Add Passkey" affordance — the parent
   * decides whether to open the register form in-place or navigate.
   */
  onAdd: () => void
}

interface RenameState {
  credentialId: string
  nickname: string
}

/**
 * Rename form rendered inside its dialog. Kept as its own component so each
 * rename target gets a fresh form instance seeded with the current nickname.
 */
function PasskeyRenameForm({ target, onClose }: { target: RenameState; onClose: () => void }) {
  const renameMutation = useFormMutation({
    mutationFn: async (vars: { credentialId: string; nickname: string }) => {
      const response = await withTimeout(
        handleRenamePasskeyCredential({
          path: { credentialId: vars.credentialId },
          body: { nickname: vars.nickname },
        })
      )
      if (response.error) {
        throw new Error(m['profile.passkey_list_rename_failed']())
      }
      return response.data
    },
    getSuccessMessage: () => m['profile.passkey_list_rename_success'](),
    invalidateQueries: [queryKeys.passkeyList()],
    onSuccess: () => {
      onClose()
    },
  })

  const form = useAppForm({
    schema: nicknameSchema,
    defaultValues: { nickname: target.nickname },
    onSubmit: async ({ value }) => {
      void renameMutation.mutate({
        credentialId: target.credentialId,
        nickname: value.nickname,
      })
    },
  })

  return (
    <AppForm>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          e.stopPropagation()
          form.handleSubmit()
        }}
        className="space-y-4"
      >
        <form.Field name="nickname">
          {(field) => (
            <div className="space-y-2">
              <Label htmlFor="rename-nickname">
                {m['profile.passkey_register_nickname_label']()}
              </Label>
              <Input
                id="rename-nickname"
                type="text"
                value={field.state.value ?? ''}
                onChange={(e) => field.handleChange(e.target.value)}
                placeholder={m['profile.passkey_list_rename_placeholder']()}
                data-testid="passkey-rename-input"
                autoFocus
              />
              {(field.state.meta.isTouched || form.state.isSubmitted) &&
                field.state.meta.errors.length > 0 && (
                  <p className="text-sm text-red-500">{getFieldErrorMessage(field.state.meta)}</p>
                )}
            </div>
          )}
        </form.Field>

        <div className="flex justify-end gap-2">
          <Button type="button" variant="outline" onClick={onClose}>
            {m['profile.passkey_list_rename_cancel']()}
          </Button>
          <Button
            type="submit"
            disabled={renameMutation.isSubmitting}
            data-testid="passkey-rename-submit-button"
          >
            {m['profile.passkey_list_rename_save']()}
          </Button>
        </div>
      </form>
    </AppForm>
  )
}

/**
 * Lists the current user's passkeys with rename and delete actions.
 *
 * - Rename: PATCH via `handleRenamePasskeyCredential` then invalidate the list.
 * - Delete: DELETE via `handleDeletePasskeyCredential`. When the credential is
 *   the LAST one, a risk-confirmation dialog is shown first (US-PK-009).
 *
 * Errors are mapped to a generic "operation failed" message — backend details
 * are never surfaced to the user.
 */
export function PasskeyList({ onAdd }: PasskeyListProps) {
  const queryClient = useQueryClient()
  const { data, isLoading } = useQuery(passkeyListQueryOptions)
  const credentials: PasskeyCredentialViewResponse[] = data?.credentials ?? []

  const [renameTarget, setRenameTarget] = useState<RenameState | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<PasskeyCredentialViewResponse | null>(null)
  // Whether the credential pending deletion is the last one (drives risk dialog).
  const isLastCredential = deleteTarget !== null && credentials.length <= 1

  const [isDeleting, setIsDeleting] = useState(false)

  const confirmDelete = async () => {
    if (!deleteTarget) return
    setIsDeleting(true)
    try {
      const response = await withTimeout(
        handleDeletePasskeyCredential({ path: { credentialId: deleteTarget.credentialId } })
      )
      if (response.error) {
        throw response.error
      }
      await queryClient.invalidateQueries({ queryKey: queryKeys.passkeyList() })
      toast.success(m['profile.passkey_list_delete_success']())
      setDeleteTarget(null)
    } catch {
      toast.error(m['profile.passkey_list_delete_failed']())
    } finally {
      setIsDeleting(false)
    }
  }

  if (isLoading) {
    return (
      <div className="space-y-2" data-testid="passkey-list">
        <p className="text-muted-foreground">{m['profile.passkey_loading']()}</p>
      </div>
    )
  }

  if (credentials.length === 0) {
    return (
      <div className="space-y-4" data-testid="passkey-empty-state">
        <p className="text-muted-foreground">{m['profile.passkey_list_empty']()}</p>
        <Button onClick={onAdd} data-testid="passkey-add-button">
          {m['profile.passkey_add_button']()}
        </Button>
      </div>
    )
  }

  return (
    <div className="space-y-4" data-testid="passkey-list">
      <div className="flex justify-end">
        <Button onClick={onAdd} data-testid="passkey-add-button">
          {m['profile.passkey_add_button']()}
        </Button>
      </div>

      <div className="space-y-3">
        {credentials.map((cred) => (
          <Card key={cred.credentialId} data-testid={`passkey-item-${cred.credentialId}`}>
            <CardContent className="space-y-3 py-4">
              <div className="flex items-start justify-between gap-2">
                <div className="space-y-1">
                  <p className="font-medium">
                    {cred.nickname ?? m['profile.passkey_register_nickname_placeholder']()}
                  </p>
                  <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-muted-foreground">
                    <span>
                      {m['profile.passkey_list_created']()} {formatDateTimeShort(cred.createdAt)}
                    </span>
                    <span>
                      {m['profile.passkey_list_last_used']()}{' '}
                      {cred.lastUsedAt
                        ? formatDateTimeShort(cred.lastUsedAt)
                        : m['profile.passkey_list_never_used']()}
                    </span>
                  </div>
                </div>
                <Badge variant={cred.backupState ? 'default' : 'secondary'}>
                  {cred.backupEligible && cred.backupState
                    ? m['profile.passkey_list_synced']()
                    : m['profile.passkey_list_device_only']()}
                </Badge>
              </div>

              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() =>
                    setRenameTarget({
                      credentialId: cred.credentialId,
                      nickname: cred.nickname ?? '',
                    })
                  }
                >
                  {m['profile.passkey_list_rename']()}
                </Button>
                <Button
                  variant="destructive"
                  size="sm"
                  onClick={() => setDeleteTarget(cred)}
                  data-testid="passkey-delete-button"
                >
                  {m['profile.passkey_list_delete']()}
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Rename dialog */}
      <Dialog open={renameTarget !== null} onOpenChange={(open) => !open && setRenameTarget(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{m['profile.passkey_list_rename_title']()}</DialogTitle>
          </DialogHeader>
          {renameTarget && (
            <PasskeyRenameForm
              key={renameTarget.credentialId}
              target={renameTarget}
              onClose={() => setRenameTarget(null)}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Delete confirmation (always shown; carries the extra risk copy when last) */}
      <Dialog open={deleteTarget !== null} onOpenChange={(open) => !open && setDeleteTarget(null)}>
        <DialogContent data-testid="passkey-delete-confirm-dialog">
          <DialogHeader>
            <DialogTitle>{m['profile.passkey_delete_confirm_title']()}</DialogTitle>
            <DialogDescription>
              {m['profile.passkey_delete_confirm_description']()}
            </DialogDescription>
          </DialogHeader>
          {isLastCredential && (
            <p className="text-sm font-medium text-red-600">
              {m['profile.passkey_delete_last_warning']()}
            </p>
          )}
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setDeleteTarget(null)}
              disabled={isDeleting}
            >
              {m['profile.passkey_delete_cancel_button']()}
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={confirmDelete}
              disabled={isDeleting}
              data-testid="passkey-delete-confirm-button"
            >
              {m['profile.passkey_delete_confirm_button']()}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
