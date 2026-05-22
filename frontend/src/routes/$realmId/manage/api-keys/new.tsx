import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const ApiKeyFormPage = lazy(() =>
  import('@/components/api-keys/api-key-form-page').then((m) => ({
    default: m.ApiKeyFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/api-keys/new')({
  component: NewApiKeyPage,
})

function NewApiKeyPage() {
  const { realmId } = Route.useParams()

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
        <ApiKeyFormPage mode="create" realmId={realmId} />
      </Suspense>
    </div>
  )
}
