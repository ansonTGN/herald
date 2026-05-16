import { createFileRoute, redirect } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { useDialogManager } from '@/hooks/use-dialog-state'
import { toast } from 'sonner'
import { queryKeys, usersQueryOptions } from '@/data/query-options'
import { usersSearchSchema, type UsersSearchParams } from '@/lib/schemas/search-params'
import { UserSearch } from '@/components/users/user-search'
import { UserTable } from '@/components/users/user-table'
import { ListPagination } from '@/components/shared'
import { CreateUserDialog } from '@/components/users/create-user-dialog'
import { EditUserDialog } from '@/components/users/edit-user-dialog'
import { UserRolesDialog } from '@/components/users/user-roles-dialog'
import { deleteUser } from '@/lib/api-generated'
import { useRealmId, useAuthStore } from '@/stores/auth-store'
import type { UserResponse } from '@/lib/api-generated'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ConfirmDeleteDialog, PageHeader } from '@/components/shared'

export const Route = createFileRoute('/$realmId/manage/users')({
  component: UsersPage,
  validateSearch: (search) => usersSearchSchema.parse(search),
  loader: ({ params }) => {
    const urlRealmId = params.realmId
    const authRealmId = useAuthStore.getState().realmId

    // Prevent cross-realm access: redirect user to their authenticated realm
    if (authRealmId && urlRealmId !== authRealmId) {
      console.warn(
        `[Users loader] Cross-realm access blocked - URL: ${urlRealmId}, Auth: ${authRealmId}`
      )
      throw redirect({
        to: '/$realmId/manage/users',
        params: { realmId: authRealmId },
      })
    }

    return { urlRealmId }
  },
})

function UsersPage() {
  const search = Route.useSearch() as UsersSearchParams
  const navigate = Route.useNavigate()
  const queryClient = useQueryClient()
  const realmId = useRealmId()
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false)
  const editDialog = useDialogManager<UserResponse>()
  const deleteDialog = useDialogManager<UserResponse>()
  const rolesDialog = useDialogManager<UserResponse>()

  const { data, isLoading, error } = useQuery(
    usersQueryOptions(realmId, {
      page: search.page,
      pageSize: search.pageSize,
      email: search.email,
    })
  )

  const deleteMutation = useMutation({
    mutationFn: (userId: string) =>
      deleteUser({
        path: { realmId, userId },
      }),
    onSuccess: () => {
      deleteDialog.close()
      queryClient.invalidateQueries({ queryKey: queryKeys.usersList(realmId) })
      toast.success('User deleted successfully')
    },
    onError: (error: Error) => {
      toast.error(error.message ?? 'Failed to delete user')
    },
  })

  function handleEdit(user: UserResponse) {
    editDialog.open(user)
  }

  function handleDelete(user: UserResponse) {
    deleteDialog.open(user)
  }

  function handleCreateUser() {
    setIsCreateDialogOpen(true)
  }

  function handleManageRoles(user: UserResponse) {
    rolesDialog.open(user)
  }

  function handleSearchChange(email: string | undefined) {
    navigate({ search: (prev) => ({ ...prev, email, page: 0 }) })
  }

  function handlePageChange(page: number) {
    navigate({ search: (prev) => ({ ...prev, page }) })
  }

  return (
    <div data-testid="users-page" className="space-y-6">
      <PageHeader
        title="Users"
        headingTestId="users-heading"
        action={{
          label: 'Add User',
          onClick: handleCreateUser,
          testId: 'create-user-button',
        }}
      />

      <Card>
        <CardHeader>
          <CardTitle>Users</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center gap-4">
            <UserSearch email={search.email} onSearchChange={handleSearchChange} />
          </div>

          {data && (
            <UserTable
              data={data.items}
              isLoading={isLoading}
              error={error ?? undefined}
              onEdit={handleEdit}
              onDelete={handleDelete}
              onManageRoles={handleManageRoles}
            />
          )}
        </CardContent>
      </Card>

      {data && (
        <ListPagination
          page={data.page}
          pageSize={data.pageSize}
          total={data.total}
          onPageChange={handlePageChange}
          testIdPrefix="user-pagination"
        />
      )}

      <CreateUserDialog
        open={isCreateDialogOpen}
        onOpenChange={setIsCreateDialogOpen}
        realmId={realmId}
      />

      {editDialog.selectedItem && (
        <EditUserDialog
          open={editDialog.isOpen}
          onOpenChange={editDialog.onOpenChange}
          realmId={realmId}
          user={editDialog.selectedItem}
        />
      )}

      {rolesDialog.selectedItem && (
        <UserRolesDialog
          open={rolesDialog.isOpen}
          onOpenChange={(v) => {
            if (!v) rolesDialog.close()
          }}
          userId={rolesDialog.selectedItem.id}
          userEmail={rolesDialog.selectedItem.email}
        />
      )}

      {deleteDialog.selectedItem && (
        <ConfirmDeleteDialog
          open={deleteDialog.isOpen}
          onOpenChange={(v) => {
            if (!v) deleteDialog.close()
          }}
          title="Delete User"
          description={`Are you sure you want to delete user "${deleteDialog.selectedItem.email}"? This action cannot be undone.`}
          onConfirm={() => deleteMutation.mutate(deleteDialog.selectedItem!.id)}
          isPending={deleteMutation.isPending}
          contentTestId="delete-user-dialog"
          confirmTestId="confirm-delete-user-button"
          cancelTestId="cancel-delete-user-button"
        />
      )}
    </div>
  )
}
