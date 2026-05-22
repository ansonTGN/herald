import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { apiKeysQueryOptions, queryKeys } from '@/data/query-options'
import { apiKeysSearchSchema } from '@/lib/schemas/search-params'
import { DeleteApiKeyDialog } from '@/components/api-keys/delete-api-key-dialog'
import { ApiKeyTable } from '@/components/api-keys/api-key-table'
import { ListPagination } from '@/components/shared'
import { Plus } from 'lucide-react'
import { useDialogManager } from '@/hooks/use-dialog-state'
import { deleteApiKey, updateApiKey } from '@/lib/api-generated'
import { useFormMutation } from '@/hooks/use-form-mutation'
import { usePermission } from '@/hooks/use-permission'
import type { ApiKeyListItem } from '@/lib/api-generated'
import type { ApiKeysSearchParams } from '@/lib/schemas/search-params'
import { Card, CardContent } from '@/components/ui/card'
import { PageHeader } from '@/components/shared'

export const Route = createFileRoute('/$realmId/manage/api-keys/')({
  component: ApiKeysPage,
  validateSearch: (search): ApiKeysSearchParams => {
    const parsed = apiKeysSearchSchema.parse(search)
    return {
      page: parsed.page,
      pageSize: parsed.pageSize,
    }
  },
})

function ApiKeysPage() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()
  const search = Route.useSearch()
  const { hasPermission } = usePermission()

  const canManage = hasPermission('api_keys.manage')

  const deleteDialog = useDialogManager<ApiKeyListItem>()

  const { data, isLoading, error } = useQuery(
    apiKeysQueryOptions(realmId, {
      page: search.page,
      pageSize: search.pageSize,
    })
  )

  const { mutate: deleteMutate } = useFormMutation({
    mutationFn: (key: ApiKeyListItem) =>
      deleteApiKey({
        path: { realmId, apiKeyId: key.id },
      }).then((response) => {
        if (response.error) throw response.error
        return response.data
      }),
    getSuccessMessage: () => `API Key deleted successfully`,
    invalidateQueries: [queryKeys.apiKeysList(realmId)],
  })

  const { mutate: toggleMutate } = useFormMutation({
    mutationFn: (key: ApiKeyListItem) =>
      updateApiKey({
        path: { realmId, apiKeyId: key.id },
        body: { enabled: !key.enabled },
      }).then((response) => {
        if (response.error) throw response.error
        return { ...key, enabled: !key.enabled }
      }),
    getSuccessMessage: (data) => `API Key "${data.name}" ${data.enabled ? 'enabled' : 'disabled'}`,
    invalidateQueries: [queryKeys.apiKeysList(realmId)],
  })

  const handlePageChange = (newPage: number) => {
    navigate({
      to: '/$realmId/manage/api-keys',
      params: { realmId },
      search: { ...search, page: newPage },
    })
  }

  return (
    <div className="space-y-6" data-testid="api-keys-page">
      <PageHeader
        title="API Keys"
        headingTestId="api-keys-heading"
        action={
          canManage
            ? {
                label: 'Add API Key',
                onClick: () =>
                  navigate({ to: '/$realmId/manage/api-keys/new', params: { realmId } }),
                testId: 'add-api-key-button',
                icon: <Plus className="h-4 w-4 mr-2" />,
              }
            : undefined
        }
      />

      <Card>
        <CardContent className="space-y-4 pt-6">
          <ApiKeyTable
            data={data?.items ?? []}
            isLoading={isLoading}
            error={error}
            onEdit={(key) =>
              navigate({
                to: '/$realmId/manage/api-keys/$apiKeyId/edit',
                params: { realmId, apiKeyId: key.id },
              })
            }
            onDelete={(key) => deleteDialog.open(key)}
            onToggleEnabled={(key) => toggleMutate(key)}
            canUpdate={canManage}
            canDelete={canManage}
          />
        </CardContent>
      </Card>

      {data && (
        <ListPagination
          page={data.page}
          pageSize={data.pageSize}
          total={data.total}
          onPageChange={handlePageChange}
          testIdPrefix="api-key-pagination"
        />
      )}

      {deleteDialog.selectedItem && (
        <DeleteApiKeyDialog
          open={deleteDialog.isOpen}
          onOpenChange={deleteDialog.onOpenChange}
          onConfirm={() => {
            if (deleteDialog.selectedItem) {
              deleteMutate(deleteDialog.selectedItem)
              deleteDialog.close()
            }
          }}
          apiKeyName={deleteDialog.selectedItem.name}
        />
      )}
    </div>
  )
}
