import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { apiKeyQueryOptions } from '@/data/query-options'

const ApiKeyFormPage = lazy(() =>
  import('@/components/api-keys/api-key-form-page').then((m) => ({
    default: m.ApiKeyFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/api-keys/$apiKeyId/edit')({
  component: EditApiKeyPage,
})

function EditApiKeyPage() {
  const { realmId, apiKeyId } = Route.useParams()
  const { data: apiKey } = useSuspenseQuery(apiKeyQueryOptions(realmId, apiKeyId))

  return (
    <div className="container max-w-4xl mx-auto py-6 px-6">
      <Suspense
        fallback={
          <div
            className="flex items-center justify-center py-12"
            data-testid="api-key-form-loading"
          >
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <ApiKeyFormPage mode="edit" realmId={realmId} apiKey={apiKey} />
      </Suspense>
    </div>
  )
}
