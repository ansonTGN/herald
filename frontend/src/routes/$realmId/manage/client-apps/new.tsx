import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const ClientAppFormPage = lazy(() =>
  import('@/components/client-apps/client-app-form-page').then((m) => ({
    default: m.ClientAppFormPage,
  }))
)

export const Route = createFileRoute('/$realmId/manage/client-apps/new')({
  component: NewClientAppPage,
})

function NewClientAppPage() {
  const { realmId } = Route.useParams()

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
        <ClientAppFormPage mode="create" realmId={realmId} />
      </Suspense>
    </div>
  )
}
