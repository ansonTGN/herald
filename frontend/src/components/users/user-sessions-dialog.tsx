import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Skeleton } from '@/components/ui/skeleton'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { ConfirmDialog } from '@/components/shared'
import { formatDateTimeShort } from '@/lib/date-utils'
import { getErrorMessage } from '@/lib/error-utils'
import { userSessionsQueryOptions } from '@/data/query-options'
import { useRevokeUserSession, useRevokeAllUserSessions } from '@/data/session-mutations'
import type { UserResponse } from '@/lib/api-generated'
import { m } from '@/paraglide/messages'

interface UserSessionsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId: string
  user: UserResponse | null
}

export function UserSessionsDialog({ open, onOpenChange, realmId, user }: UserSessionsDialogProps) {
  const [pendingFamilyId, setPendingFamilyId] = useState<string | null>(null)
  const [showRevokeAllConfirm, setShowRevokeAllConfirm] = useState(false)

  const userId = user?.id ?? ''

  const revokeOneMutation = useRevokeUserSession(realmId, userId)
  const revokeAllMutation = useRevokeAllUserSessions(realmId, userId)

  const handleConfirmRevoke = async () => {
    if (!pendingFamilyId) return
    try {
      await revokeOneMutation.mutateAsync(pendingFamilyId)
      setPendingFamilyId(null)
      toast.success(m['users.sessions.revoke_success']())
    } catch (error) {
      toast.error(getErrorMessage(error) ?? m['users.sessions.revoke_failed']())
    }
  }

  const handleConfirmRevokeAll = async () => {
    try {
      const data = await revokeAllMutation.mutateAsync()
      setShowRevokeAllConfirm(false)
      toast.success(m['users.sessions.revoke_all_success']({ count: data.revokedCount }))
    } catch (error) {
      toast.error(getErrorMessage(error) ?? m['users.sessions.revoke_all_failed']())
    }
  }

  const { data, isLoading, isError, refetch } = useQuery({
    ...userSessionsQueryOptions(realmId, userId),
    enabled: open && !!user,
  })

  const sessions = data ?? []
  const hasSessions = sessions.length > 0

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[640px]" data-testid="user-sessions-dialog">
        <DialogHeader>
          <DialogTitle>{user ? m['users.sessions.title']({ email: user.email }) : ''}</DialogTitle>
          <DialogDescription>{m['users.sessions.description']()}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {hasSessions && (
            <div className="flex justify-end">
              <Button
                variant="destructive"
                onClick={() => setShowRevokeAllConfirm(true)}
                data-testid="user-sessions-revoke-all-button"
              >
                {m['users.sessions.revoke_all_button']()}
              </Button>
            </div>
          )}

          {isLoading ? (
            <div className="space-y-2">
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
              <Skeleton className="h-8 w-full" />
            </div>
          ) : isError ? (
            <div className="flex flex-col items-center gap-3 py-8">
              <p className="text-sm text-destructive">{m['users.sessions.error']()}</p>
              <Button
                variant="outline"
                onClick={() => refetch()}
                data-testid="user-sessions-retry-button"
              >
                {m['users.sessions.retry']()}
              </Button>
            </div>
          ) : !hasSessions ? (
            <p className="py-8 text-center text-sm text-muted-foreground">
              {m['users.sessions.empty']()}
            </p>
          ) : (
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{m['users.sessions.table_client_app']()}</TableHead>
                    <TableHead>{m['users.sessions.table_user_agent']()}</TableHead>
                    <TableHead>{m['users.sessions.table_client_ip']()}</TableHead>
                    <TableHead>{m['users.sessions.table_created_at']()}</TableHead>
                    <TableHead className="text-right">
                      {m['users.sessions.table_actions']()}
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sessions.map((session, index) => (
                    <TableRow key={session.familyId}>
                      <TableCell className="font-medium">
                        {session.clientAppName ?? session.clientAppId}
                      </TableCell>
                      <TableCell className="max-w-[200px] truncate">
                        {session.userAgent ?? '-'}
                      </TableCell>
                      <TableCell>{session.clientIp ?? '-'}</TableCell>
                      <TableCell>
                        {session.createdAt ? formatDateTimeShort(session.createdAt) : '-'}
                      </TableCell>
                      <TableCell className="text-right">
                        <Button
                          variant="destructive"
                          size="sm"
                          onClick={() => setPendingFamilyId(session.familyId)}
                          data-testid={`user-sessions-table-${index}-revoke-button`}
                        >
                          {m['users.sessions.revoke_button']()}
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            {m['common.close']()}
          </Button>
        </DialogFooter>
      </DialogContent>

      {/* Revoke-one confirmation */}
      <ConfirmDialog
        open={pendingFamilyId !== null}
        onOpenChange={(v) => {
          if (!v) setPendingFamilyId(null)
        }}
        title={m['users.sessions.revoke_title']()}
        description={m['users.sessions.revoke_description']()}
        confirmLabel={m['users.sessions.revoke_confirm']()}
        onConfirm={handleConfirmRevoke}
        isPending={revokeOneMutation.isPending}
        contentTestId="user-sessions-revoke-confirm-dialog"
        cancelTestId="user-sessions-revoke-cancel-button"
        confirmTestId="user-sessions-revoke-confirm-button"
      />

      {/* Revoke-all confirmation */}
      <ConfirmDialog
        open={showRevokeAllConfirm}
        onOpenChange={setShowRevokeAllConfirm}
        title={m['users.sessions.revoke_all_title']()}
        description={m['users.sessions.revoke_all_description']()}
        confirmLabel={m['users.sessions.revoke_all_confirm']()}
        onConfirm={handleConfirmRevokeAll}
        isPending={revokeAllMutation.isPending}
        contentTestId="user-sessions-revoke-all-confirm-dialog"
        cancelTestId="user-sessions-revoke-all-cancel-button"
        confirmTestId="user-sessions-revoke-all-confirm-button"
      />
    </Dialog>
  )
}
