import { createFileRoute, redirect } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { toast } from 'sonner'
import { queryKeys, usersQueryOptions } from '@/data/query-options'
import { usersSearchSchema, type UsersSearchParams } from '@/lib/schemas/search-params'
import { UserSearch } from '@/components/users/user-search'
import { UserTable } from '@/components/users/user-table'
import { UserPagination } from '@/components/users/user-pagination'
import { CreateUserDialog } from '@/components/users/create-user-dialog'
import { EditUserDialog } from '@/components/users/edit-user-dialog'
import { UserRolesDialog } from '@/components/users/user-roles-dialog'
import { deleteUser } from '@/lib/api-generated'
import { useRealmId, useAuthStore } from '@/stores/auth-store'
import type { UserResponse } from '@/lib/api-generated'
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
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false)
  const [isRolesDialogOpen, setIsRolesDialogOpen] = useState(false)
  const [editingUser, setEditingUser] = useState<UserResponse | null>(null)
  const [managingRolesUser, setManagingRolesUser] = useState<UserResponse | null>(null)
  const [deletingUser, setDeletingUser] = useState<UserResponse | null>(null)
  const [searchEmail, setSearchEmail] = useState<string | undefined>(undefined)

  const { data, isLoading, error } = useQuery(
    usersQueryOptions(realmId, {
      page: search.page,
      pageSize: search.pageSize,
      email: searchEmail,
    })
  )

  const deleteMutation = useMutation({
    mutationFn: (userId: string) =>
      deleteUser({
        path: { realmId, userId },
      }),
    onSuccess: () => {
      setDeletingUser(null)
      queryClient.invalidateQueries({ queryKey: queryKeys.usersList(realmId) })
      toast.success('User deleted successfully')
    },
    onError: (error: Error) => {
      toast.error(error.message ?? 'Failed to delete user')
    },
  })

  function handleEdit(user: UserResponse) {
    setEditingUser(user)
    setIsEditDialogOpen(true)
  }

  function handleDelete(user: UserResponse) {
    setDeletingUser(user)
  }

  function handleCreateUser() {
    setIsCreateDialogOpen(true)
  }

  function handleManageRoles(user: UserResponse) {
    setManagingRolesUser(user)
    setIsRolesDialogOpen(true)
  }

  function handleSearchChange(email: string | undefined) {
    setSearchEmail(email)
  }

  function handlePageChange(page: number) {
    navigate({ search: (prev) => ({ ...prev, page }) })
  }

  return (
    <div data-testid="users-page" className="space-y-6">
      <PageHeader
        title="Users"
        description="Manage user accounts and permissions"
        headingTestId="users-heading"
        action={{
          label: 'Add User',
          onClick: handleCreateUser,
          testId: 'create-user-button',
        }}
      />

      <div className="flex items-center gap-4">
        <UserSearch email={searchEmail} onSearchChange={handleSearchChange} />
      </div>

      {data && (
        <>
          <UserTable
            data={data.items}
            isLoading={isLoading}
            error={error ?? undefined}
            onEdit={handleEdit}
            onDelete={handleDelete}
            onManageRoles={handleManageRoles}
          />
          <UserPagination pagination={data} onPageChange={handlePageChange} />
        </>
      )}

      <CreateUserDialog
        open={isCreateDialogOpen}
        onOpenChange={setIsCreateDialogOpen}
        realmId={realmId}
      />

      {editingUser && (
        <EditUserDialog
          open={isEditDialogOpen}
          onOpenChange={setIsEditDialogOpen}
          realmId={realmId}
          user={editingUser}
        />
      )}

      {managingRolesUser && (
        <UserRolesDialog
          open={isRolesDialogOpen}
          onOpenChange={(open) => {
            setIsRolesDialogOpen(open)
            if (!open) setManagingRolesUser(null)
          }}
          userId={managingRolesUser.id}
          userEmail={managingRolesUser.email}
        />
      )}

      {deletingUser && (
        <ConfirmDeleteDialog
          open={!!deletingUser}
          onOpenChange={(open) => {
            if (!open) setDeletingUser(null)
          }}
          title="Delete User"
          description={`Are you sure you want to delete user "${deletingUser.email}"? This action cannot be undone.`}
          onConfirm={() => deleteMutation.mutate(deletingUser.id)}
          isPending={deleteMutation.isPending}
          contentTestId="delete-user-dialog"
          confirmTestId="confirm-delete-user-button"
          cancelTestId="cancel-delete-user-button"
        />
      )}
    </div>
  )
}
