import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { apiKeyQueryOptions } from '@/data/query-options'
import { useLastPathSegment, useResolvedRealmId } from '@/lib/realm-routing'

const ApiKeyFormPage = lazy(() =>
  import('@/components/api-keys/api-key-form-page').then((m) => ({
    default: m.ApiKeyFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/api-keys/$apiKeyId/edit')({
  component: EditApiKeyPage,
})

export function EditApiKeyPage() {
  const realmId = useResolvedRealmId()
  const apiKeyId = useLastPathSegment(1)
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
