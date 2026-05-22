import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const ApiKeyRevealPage = lazy(() =>
  import('@/components/api-keys/api-key-reveal-page').then((m) => ({
    default: m.ApiKeyRevealPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/api-keys/reveal')({
  component: RevealApiKeyPage,
})

function RevealApiKeyPage() {
  const { realmId } = Route.useParams()

  return (
    <div className="container max-w-4xl mx-auto py-6 px-6">
      <Suspense
        fallback={
          <div
            className="flex items-center justify-center py-12"
            data-testid="api-key-reveal-loading"
          >
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <ApiKeyRevealPage realmId={realmId} />
      </Suspense>
    </div>
  )
}
