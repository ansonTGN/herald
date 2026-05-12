import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const ClientAppWizard = lazy(() =>
  import('@/components/client-apps/client-app-wizard').then((m) => ({
    default: m.ClientAppWizard,
  }))
)

export const Route = createFileRoute('/$realmId/manage/client-apps/new')({
  component: NewClientAppPage,
})

function NewClientAppPage() {
  const { realmId } = Route.useParams()

  return (
    <div className="container max-w-3xl mx-auto py-12 px-6">
      <Suspense
        fallback={
          <div className="flex items-center justify-center py-12" data-testid="wizard-loading">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        }
      >
        <ClientAppWizard mode="create" realmId={realmId} />
      </Suspense>
    </div>
  )
}
