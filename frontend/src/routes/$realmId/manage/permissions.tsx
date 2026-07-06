import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useRealmId } from '@/stores/auth-store'
import { adminPermissionsQueryOptions } from '@/data/query-options'
import { PermissionTable } from '@/components/permissions/permission-table'
import { CreatePermissionDialog } from '@/components/permissions/create-permission-dialog'
import { Card, CardContent } from '@/components/ui/card'
import { PageHeader } from '@/components/shared'
import { useState } from 'react'
import { m } from '@/paraglide/messages'

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
    ...adminPermissionsQueryOptions(realmId),
  })

  return (
    <div className="space-y-6" data-testid="permissions-page">
      <PageHeader
        title={m['permissions.page_title']()}
        action={{
          label: m['permissions.add_button'](),
          onClick: () => setCreateDialogOpen(true),
          testId: 'permission-create-button',
        }}
      />

      <Card>
        <CardContent className="pt-6">
          <PermissionTable permissions={permissions ?? []} isLoading={isLoading} error={error} />
        </CardContent>
      </Card>

      <CreatePermissionDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
        realmId={realmId}
      />
    </div>
  )
}
