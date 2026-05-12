import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useRealmId } from '@/stores/auth-store'
import { permissionsQueryOptions } from '@/data/query-options'
import { PermissionTable } from '@/components/permissions/permission-table'
import { CreatePermissionDialog } from '@/components/permissions/create-permission-dialog'
import { PageHeader } from '@/components/shared'
import { useState } from 'react'

export const Route = createFileRoute('/$realmId/manage/permissions')({
  component: PermissionsPage,
})

function PermissionsPage() {
  const realmId = useRealmId()
  const [createDialogOpen, setCreateDialogOpen] = useState(false)

  const {
    data: permissions,
    isLoading,
    error,
  } = useQuery({
    ...permissionsQueryOptions(realmId),
  })

  return (
    <div className="space-y-6" data-testid="permissions-page">
      <PageHeader
        title="Permissions"
        description="Manage permission definitions for your realm"
        action={{
          label: 'Add Permission',
          onClick: () => setCreateDialogOpen(true),
          testId: 'permission-create-button',
        }}
      />

      <PermissionTable permissions={permissions ?? []} isLoading={isLoading} error={error} />

      <CreatePermissionDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
        realmId={realmId}
      />
    </div>
  )
}
