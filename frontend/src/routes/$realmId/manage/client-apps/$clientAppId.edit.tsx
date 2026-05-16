import { createFileRoute } from '@tanstack/react-router'
import { useSuspenseQuery } from '@tanstack/react-query'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'
import { clientAppQueryOptions } from '@/data/query-options'

const ClientAppFormPage = lazy(() =>
  import('@/components/client-apps/client-app-form-page').then((m) => ({
    default: m.ClientAppFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/client-apps/$clientAppId/edit')({
  component: EditClientAppPage,
})

function EditClientAppPage() {
  const { realmId, clientAppId } = Route.useParams()
  const { data: clientApp } = useSuspenseQuery(clientAppQueryOptions(realmId, clientAppId))

  return (
    <div className="container max-w-4xl mx-auto py-6 px-6">
      <Suspense
        fallback={
          <div
            className="flex items-center justify-center py-12"
            data-testid="client-app-form-loading"
          >
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <ClientAppFormPage mode="edit" realmId={realmId} clientApp={clientApp} />
      </Suspense>
    </div>
  )
}
