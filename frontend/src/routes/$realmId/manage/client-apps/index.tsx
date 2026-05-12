import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { clientAppsQueryOptions, queryKeys } from '@/data/query-options'
import { clientAppsSearchSchema } from '@/lib/schemas/search-params'
import { DeleteClientAppDialog } from '@/components/client-apps/delete-client-app-dialog'
import { ClientAppTable } from '@/components/client-apps/client-app-table'
import { ClientAppPagination } from '@/components/client-apps/client-app-pagination'
import { Plus } from 'lucide-react'
import { useState } from 'react'
import { deleteClientApp, updateClientApp } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { usePermission } from '@/hooks/use-permission'
import type { ClientAppItem } from '@/lib/api-generated'
import type { ClientAppsSearchParams } from '@/lib/schemas/search-params'
import { PageHeader } from '@/components/shared'

export const Route = createFileRoute('/$realmId/manage/client-apps/')({
  component: ClientAppsPage,
  validateSearch: (search): ClientAppsSearchParams => {
    const parsed = clientAppsSearchSchema.parse(search)
    return {
      page: parsed.page,
      pageSize: parsed.pageSize,
    }
  },
})

function ClientAppsPage() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()
  const search = Route.useSearch()
  const { hasPermission } = usePermission()

  const canCreate = hasPermission('clients.manage')
  const canUpdate = hasPermission('clients.manage')
  const canDelete = hasPermission('clients.manage')

  const [deleteDialog, setDeleteDialog] = useState<{
    open: boolean
    clientApp: ClientAppItem | null
  }>({
    open: false,
    clientApp: null,
  })

  const { data, isLoading, error } = useQuery(
    clientAppsQueryOptions(realmId, {
      page: search.page,
      pageSize: search.pageSize,
    })
  )

  const { mutate: deleteMutate } = useFormMutation({
    mutationFn: (app: ClientAppItem) =>
      deleteClientApp({
        path: { realmId, clientAppId: app.id },
      }).then((response) => {
        if (response.error) throw response.error
        return response.data
      }),
    getSuccessMessage: () => `Client App deleted successfully`,
    invalidateQueries: [queryKeys.clientAppsList(realmId)],
  })

  const { mutate: toggleMutate } = useFormMutation({
    mutationFn: (app: ClientAppItem) =>
      updateClientApp({
        path: { realmId, clientAppId: app.id },
        body: { enabled: !app.enabled },
      }).then((response) => {
        if (response.error) throw response.error
        return { ...app, enabled: !app.enabled }
      }),
    getSuccessMessage: (data) =>
      `Client App "${data.name}" ${data.enabled ? 'enabled' : 'disabled'}`,
    invalidateQueries: [queryKeys.clientAppsList(realmId)],
  })

  const handlePageChange = (newPage: number) => {
    navigate({
      to: '/$realmId/manage/client-apps',
      params: { realmId },
      search: { ...search, page: newPage },
    })
  }

  return (
    <div className="space-y-4" data-testid="client-apps-page">
      <PageHeader
        title="Client Apps"
        description="Manage OAuth 2.0 client applications"
        headingTestId="client-apps-heading"
        action={
          canCreate
            ? {
                label: 'Add Client App',
                onClick: () => {
                  navigate({
                    to: '/$realmId/manage/client-apps/new',
                    params: { realmId },
                  })
                },
                testId: 'add-client-app-button',
                icon: <Plus className="h-4 w-4 mr-2" />,
              }
            : undefined
        }
      />

      <ClientAppTable
        data={data?.items ?? []}
        isLoading={isLoading}
        error={error}
        onEdit={(app) => {
          navigate({
            to: '/$realmId/manage/client-apps/$clientAppId/edit',
            params: { realmId, clientAppId: app.id },
          })
        }}
        onDelete={(app) => {
          setDeleteDialog({ open: true, clientApp: app })
        }}
        onToggleEnabled={(app) => toggleMutate(app)}
        canUpdate={canUpdate}
        canDelete={canDelete}
      />

      {data && <ClientAppPagination pagination={data} onPageChange={handlePageChange} />}

      {deleteDialog.clientApp && (
        <DeleteClientAppDialog
          open={deleteDialog.open}
          onOpenChange={(open) => setDeleteDialog({ ...deleteDialog, open })}
          onConfirm={() => {
            if (deleteDialog.clientApp) {
              deleteMutate(deleteDialog.clientApp)
              setDeleteDialog({ open: false, clientApp: null })
            }
          }}
          clientAppName={deleteDialog.clientApp.name}
        />
      )}
    </div>
  )
}
