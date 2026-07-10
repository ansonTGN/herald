import { createFileRoute } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'
import { Loader2 } from 'lucide-react'

const TotpSetupPage = lazy(() =>
  import('@/components/auth/totp-setup-page').then((m) => ({
    default: m.TotpSetupPage,
  }))
)

export const Route = createFileRoute('/$realmId/user/security/totp-setup')({
  component: TotpSetupPageRoute,
})

export function TotpSetupPageRoute() {
  return (
    <Suspense
      fallback={
        <div
          className="flex items-center justify-center py-12"
          data-testid="totp-setup-page-loading"
        >
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      }
    >
      <TotpSetupPage />
    </Suspense>
  )
}
