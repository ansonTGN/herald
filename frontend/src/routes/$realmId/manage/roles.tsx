import { createFileRoute } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useRealmId } from '@/stores/auth-store'
import { rolesQueryOptions } from '@/data/query-options'
import { RoleTable } from '@/components/roles/role-table'
import { CreateRoleDialog } from '@/components/roles/create-role-dialog'
import { Card, CardContent } from '@/components/ui/card'
import { PageHeader } from '@/components/shared'
import { useState } from 'react'
import { m } from '@/paraglide/messages'

export const Route = createFileRoute('/$realmId/manage/roles')({
  component: RolesPage,
})

function RolesPage() {
  const realmId = useRealmId()
  const [createDialogOpen, setCreateDialogOpen] = useState(false)

  const {
    data: roles,
    isLoading,
    error,
  } = useQuery({
    ...rolesQueryOptions(realmId),
  })

  return (
    <div className="space-y-6" data-testid="roles-page">
      <PageHeader
        title={m['roles.page_title']()}
        headingTestId="roles-heading"
        action={{
          label: m['roles.add_button'](),
          onClick: () => setCreateDialogOpen(true),
          testId: 'role-create-button',
        }}
      />

      <Card>
        <CardContent className="pt-6">
          <RoleTable roles={roles ?? []} isLoading={isLoading} error={error} />
        </CardContent>
      </Card>

      <CreateRoleDialog
        open={createDialogOpen}
        onOpenChange={setCreateDialogOpen}
        realmId={realmId}
      />
    </div>
  )
}
