import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import { Loader2 } from 'lucide-react'

import { AppForm, useAppForm } from '@/components/ui/tanstack-form'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { UserSearch } from '@/components/users/user-search'
import { NumberField } from '@/components/shared/form-fields/number-field'
import { TextareaField } from '@/components/shared/form-fields/textarea-field'
import { getFieldErrorMessage } from '@/lib/form-utils'
import { grantPointsSchema } from '@/lib/schemas/points-forms'
import type { GrantPointsFormData } from '@/lib/schemas/points-forms'
import { usersQueryOptions } from '@/data/query-options'
import { useGrantPoints } from '@/data/grant-points-mutations'
import { useEnabledBuckets } from '@/data/use-buckets'
import { usePermission } from '@/hooks/use-permission'
import { PERMISSION } from '@/lib/constants/auth-constants'
import { getErrorMessage } from '@/lib/error-utils'
import type { UserResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

interface GrantPointsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string
}

export function GrantPointsDialog({ open, onOpenChange, realmId }: GrantPointsDialogProps) {
  const { hasPermission } = usePermission()
  const canManagePoints = hasPermission(PERMISSION.POINTS_MANAGE)

  const [selectedUser, setSelectedUser] = useState<UserResponse | null>(null)
  const [userSearchQuery, setUserSearchQuery] = useState('')
  const [confirmOpen, setConfirmOpen] = useState(false)
  const [serverError, setServerError] = useState<string | null>(null)
  const [pendingGrant, setPendingGrant] = useState<GrantPointsFormData | null>(null)
  const [isPermanent, setIsPermanent] = useState(true)

  const grantMutation = useGrantPoints(realmId)

  // Enabled buckets for the mandatory Target bucket Select.
  // Disabled buckets are filtered out so admins can't target them.
  const { buckets } = useEnabledBuckets(realmId)

  // User search query -- enabled only when query is non-empty
  const { data: usersData, isLoading: isSearching } = useQuery({
    ...usersQueryOptions(realmId, { email: userSearchQuery }),
    enabled: userSearchQuery.length > 0,
  })

  const users = usersData?.items ?? []
  const showNoResults = userSearchQuery.length > 0 && !isSearching && users.length === 0

  const form = useAppForm({
    schema: grantPointsSchema,
    defaultValues: {
      userId: '',
      amount: 1,
      reason: '',
      // No default bucket — grant must explicitly target a bucket.
      bucketId: '',
      validityDays: null as number | null,
    },
    onSubmit: async ({ value }) => {
      setPendingGrant(value as GrantPointsFormData)
      setConfirmOpen(true)
    },
  })

  const handleUserSelect = (user: UserResponse) => {
    setSelectedUser(user)
    form.setFieldValue('userId', user.id)
    setServerError(null)
  }

  const handleClearUser = () => {
    setSelectedUser(null)
    form.setFieldValue('userId', '')
    setUserSearchQuery('')
  }

  const handleSearchChange = (email: string | undefined) => {
    setUserSearchQuery(email ?? '')
  }

  const resetDialogState = () => {
    setSelectedUser(null)
    setUserSearchQuery('')
    setConfirmOpen(false)
    setServerError(null)
    setPendingGrant(null)
    setIsPermanent(true)
    form.reset()
  }

  const handleConfirmGrant = async () => {
    if (!selectedUser || !pendingGrant) return
    try {
      await grantMutation.mutateAsync({
        userId: selectedUser.id,
        amount: pendingGrant.amount,
        reason: pendingGrant.reason,
        validityDays: pendingGrant.validityDays ?? null,
        bucketId: pendingGrant.bucketId,
      })
      toast.success(
        m['points.grant_dialog_granted_success']({
          amount: pendingGrant.amount,
          email: selectedUser.email,
        })
      )
      onOpenChange(false)
      resetDialogState()
    } catch (error) {
      // Defense-in-depth: the schema should block an empty/missing bucket,
      // but if the backend still rejects with 400 `grant_bucket_required`
      // (e.g. bucket disabled server-side between select and submit), surface
      // the specific message instead of a generic bad-request string.
      if (
        typeof error === 'object' &&
        error !== null &&
        'code' in error &&
        error.code === 'grant_bucket_required'
      ) {
        setServerError(m['points.grant_dialog_bucket_required_error']())
      } else {
        setServerError(getErrorMessage(error))
      }
      setConfirmOpen(false)
    }
  }

  const handleOpenChange = (nextOpen: boolean) => {
    if (!nextOpen) {
      resetDialogState()
    }
    onOpenChange(nextOpen)
  }

  // Permission gate -- defense-in-depth
  if (!canManagePoints) {
    return (
      <Dialog open={open} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-[425px]" data-testid="grant-points-form-dialog">
          <DialogHeader>
            <DialogTitle>{m['points.grant_dialog_title']()}</DialogTitle>
            <DialogDescription>{m['points.grant_dialog_no_permission']()}</DialogDescription>
          </DialogHeader>
        </DialogContent>
      </Dialog>
    )
  }

  return (
    <>
      {/* Main grant points form dialog */}
      <Dialog open={open} onOpenChange={handleOpenChange}>
        <DialogContent className="sm:max-w-[425px]" data-testid="grant-points-form-dialog">
          <DialogHeader>
            <DialogTitle>{m['points.grant_dialog_title']()}</DialogTitle>
            <DialogDescription>{m['points.grant_dialog_description']()}</DialogDescription>
          </DialogHeader>

          <AppForm>
            <form
              onSubmit={(e) => {
                e.preventDefault()
                e.stopPropagation()
                form.handleSubmit()
              }}
              className="space-y-4"
            >
              {/* Server error alert */}
              {serverError && (
                <Alert variant="destructive" data-testid="grant-points-error-message">
                  <AlertDescription>{serverError}</AlertDescription>
                </Alert>
              )}

              {/* User selection */}
              <div className="space-y-2">
                <Label>{m['points.grant_dialog_user_label']()}</Label>
                {selectedUser ? (
                  <div className="flex items-center justify-between rounded-md border px-3 py-2">
                    <div>
                      <p className="text-sm font-medium">{selectedUser.email}</p>
                      {selectedUser.nickname && (
                        <p className="text-xs text-muted-foreground">{selectedUser.nickname}</p>
                      )}
                    </div>
                    <Button type="button" variant="ghost" size="sm" onClick={handleClearUser}>
                      {m['points.grant_dialog_user_change']()}
                    </Button>
                  </div>
                ) : (
                  <>
                    <div data-testid="grant-points-user-search-input">
                      <UserSearch onSearchChange={handleSearchChange} />
                    </div>
                    {isSearching && (
                      <div className="flex items-center gap-2 py-2 text-sm text-muted-foreground">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        {m['points.grant_dialog_user_searching']()}
                      </div>
                    )}
                    {showNoResults && (
                      <p className="py-2 text-sm text-muted-foreground">
                        {m['points.grant_dialog_user_no_results']()}
                      </p>
                    )}
                    {users.length > 0 && !selectedUser && (
                      <div className="max-h-40 overflow-y-auto rounded-md border">
                        {users.map((user) => (
                          <button
                            key={user.id}
                            type="button"
                            className="flex w-full items-center gap-2 px-3 py-2 text-left text-sm hover:bg-accent"
                            onClick={() => handleUserSelect(user)}
                          >
                            <div>
                              <p className="font-medium">{user.email}</p>
                              {user.nickname && (
                                <p className="text-xs text-muted-foreground">{user.nickname}</p>
                              )}
                            </div>
                          </button>
                        ))}
                      </div>
                    )}
                  </>
                )}
                {/* Hidden userId field for validation */}
                <form.Field name="userId">
                  {(field) => <input type="hidden" value={field.state.value} />}
                </form.Field>
                {/* Show userId validation error when no user is selected */}
                {(() => {
                  const userIdMeta = form.getFieldMeta('userId')
                  const userIdErrors = userIdMeta?.errors
                  return (
                    (form.state.isSubmitted || form.state.values.userId === '') &&
                    !selectedUser &&
                    userIdErrors &&
                    userIdErrors.length > 0 && (
                      <p className="text-sm text-destructive" role="alert">
                        {userIdErrors[0] ? getFieldErrorMessage({ errors: userIdErrors }) : null}
                      </p>
                    )
                  )
                })()}
              </div>

              {/* Target Bucket — REQUIRED, no default.
                  Mirrors the TransactionFilters Select pattern.
                  Options come from useEnabledBuckets so disabled buckets are
                  not selectable. */}
              <form.Field name="bucketId">
                {(field) => (
                  <div className="space-y-2">
                    <Label htmlFor="grant-bucket">{m['points.grant_dialog_bucket_label']()}</Label>
                    <Select
                      value={field.state.value}
                      onValueChange={(value) => field.handleChange(value)}
                    >
                      <SelectTrigger id="grant-bucket" data-testid="grant-points-bucket-select">
                        <SelectValue placeholder={m['points.grant_dialog_bucket_placeholder']()} />
                      </SelectTrigger>
                      <SelectContent>
                        {buckets.map((bucket) => (
                          <SelectItem key={bucket.id} value={bucket.id}>
                            {bucket.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                    {(field.state.meta.isTouched || form.state.isSubmitted) &&
                      field.state.meta.errors.length > 0 && (
                        <p className="text-sm text-destructive" role="alert">
                          {getFieldErrorMessage(field.state.meta)}
                        </p>
                      )}
                  </div>
                )}
              </form.Field>

              {/* Amount */}
              <NumberField
                form={form}
                name="amount"
                label={m['points.grant_dialog_amount_label']()}
                inputId="grant-amount"
                dataTestId="grant-points-amount-input"
                min={1}
                required
              />

              {/* Validity section */}
              <div className="space-y-3">
                <form.Field name="validityDays">
                  {(field) => (
                    <div className="space-y-2">
                      <Label htmlFor="grant-validity-days">
                        {m['points.grant_dialog_validity_label']()}
                      </Label>
                      <Input
                        id="grant-validity-days"
                        type="number"
                        min="1"
                        placeholder="e.g. 30"
                        value={field.state.value ?? ''}
                        onChange={(e) => {
                          const val = e.target.value
                          field.handleChange(val ? parseInt(val) : null)
                        }}
                        disabled={isPermanent}
                        data-testid="grant-points-validity-days-input"
                      />
                      {(field.state.meta.isTouched || form.state.isSubmitted) &&
                        field.state.meta.errors.length > 0 && (
                          <p className="text-sm text-destructive" role="alert">
                            {getFieldErrorMessage(field.state.meta)}
                          </p>
                        )}
                    </div>
                  )}
                </form.Field>

                <div className="flex items-center gap-2">
                  <Switch
                    id="grant-permanent"
                    checked={isPermanent}
                    onCheckedChange={(checked) => {
                      setIsPermanent(checked)
                      form.setFieldValue('validityDays', checked ? null : 30)
                    }}
                    data-testid="grant-points-permanent-toggle"
                  />
                  <Label htmlFor="grant-permanent" className="cursor-pointer">
                    {m['points.grant_dialog_permanent_label']()}
                  </Label>
                </div>
              </div>

              {/* Reason */}
              <TextareaField
                form={form}
                name="reason"
                label={m['points.grant_dialog_reason_label']()}
                inputId="grant-reason"
                dataTestId="grant-points-reason-input"
                placeholder={m['points.grant_dialog_reason_placeholder']()}
                rows={3}
                required
              />

              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => handleOpenChange(false)}
                  data-testid="grant-points-cancel-button"
                >
                  {m['common.cancel']()}
                </Button>
                <Button
                  type="submit"
                  disabled={grantMutation.isPending}
                  data-testid="grant-points-submit-button"
                >
                  {m['points.grant_dialog_review_button']()}
                </Button>
              </DialogFooter>
            </form>
          </AppForm>
        </DialogContent>
      </Dialog>

      {/* Confirmation dialog (sibling, not nested) */}
      <Dialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <DialogContent className="sm:max-w-[425px]" data-testid="grant-points-confirm-dialog">
          <DialogHeader>
            <DialogTitle>{m['points.grant_dialog_confirm_title']()}</DialogTitle>
            <DialogDescription>{m['points.grant_dialog_confirm_description']()}</DialogDescription>
          </DialogHeader>

          <div className="space-y-3 rounded-md border p-4 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                {m['points.grant_dialog_confirm_user']()}
              </span>
              <span className="font-medium">
                {selectedUser?.nickname
                  ? `${selectedUser.nickname} (${selectedUser.email})`
                  : selectedUser?.email}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                {m['points.grant_dialog_confirm_bucket']()}
              </span>
              <span className="font-medium">
                {buckets.find((b) => b.id === pendingGrant?.bucketId)?.name ??
                  pendingGrant?.bucketId}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                {m['points.grant_dialog_confirm_amount']()}
              </span>
              <span className="font-medium">{(pendingGrant?.amount ?? 0).toLocaleString()}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                {m['points.grant_dialog_confirm_validity']()}
              </span>
              <span className="font-medium">
                {pendingGrant?.validityDays
                  ? m['points.grant_dialog_confirm_days']({ days: pendingGrant.validityDays })
                  : m['points.grant_dialog_confirm_permanent']()}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">
                {m['points.grant_dialog_confirm_reason']()}
              </span>
              <span className="font-medium">{pendingGrant?.reason}</span>
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setConfirmOpen(false)}
              disabled={grantMutation.isPending}
            >
              {m['common.cancel']()}
            </Button>
            <Button
              type="button"
              onClick={handleConfirmGrant}
              disabled={grantMutation.isPending}
              data-testid="grant-points-confirm-button"
            >
              {grantMutation.isPending
                ? m['points.grant_dialog_confirm_granting']()
                : m['points.grant_dialog_confirm_button']()}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
