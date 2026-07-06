import {
  createRootRouteWithContext,
  Outlet,
  redirect,
  isRedirect,
  useRouter,
} from '@tanstack/react-router'
import { Toaster } from '@/components/ui/sonner'
import { useQuery, useMutation, useQueryClient, type QueryClient } from '@tanstack/react-query'
import { initializeAuth, checkAdminPermission, logoutFlow } from '@/lib/auth-utils'
import { useIsAuthenticated } from '@/stores/auth-store'
import { ReconsentDialog } from '@/components/legal/ReconsentDialog'
import {
  consentStatusQueryOptions,
  recordConsentMutation,
  toRecordConsentRequestFromStatus,
  queryKeys,
} from '@/data/query-options'
import { lazy, Suspense } from 'react'

const Devtools = import.meta.env.DEV
  ? lazy(() => import('@/components/devtools').then((m) => ({ default: m.Devtools })))
  : () => null

type RouterContext = {
  queryClient: QueryClient
}

export const Route = createRootRouteWithContext<RouterContext>()({
  // Unified loader - preloads auth data for all routes and handles authentication/permissions
  // Uses Zustand store instead of React Query cache for synchronous state updates
  loader: async ({ location }) => {
    const pathname = location.pathname

    // Extract realmId from pathname (first path segment after /)
    const pathSegments = pathname.split('/').filter(Boolean)
    const realmId = pathSegments[0] || 'admin' // Default to 'admin' if no realmId in path

    // Route type detection
    const isRootPath = pathname === '/'
    const isRealmRootPath = pathname.match(/^\/[^/]+\/?$/) // /admin, /admin/, /user, /user/, etc. (realm root with optional trailing slash)
    const isAuthRoute = pathname.match(/^\/[^/]+\/auth\//) // /admin/auth/, /user/auth/, etc.
    const isLegalRoute = pathname.match(/^\/[^/]+\/legal\//) // /admin/legal/, /user/legal/, etc.
    const isManageRoute = pathname.match(/^\/[^/]+\/manage/)

    try {
      // Initialize auth and populate Zustand store
      const result = await initializeAuth(realmId)
      const authenticated = result.authenticated

      // Route redirect logic using Zustand state

      // Root path: redirect based on auth status and permissions
      if (isRootPath) {
        if (!authenticated) {
          // Redirect to login page with return URL
          throw redirect({
            to: '/$realmId/auth/login',
            params: { realmId },
            search: { redirect: '/' },
          })
        }

        // Redirect to appropriate page based on permissions (from Zustand state)
        const targetPath = checkAdminPermission() ? '/$realmId/manage' : '/$realmId/user/profile'
        throw redirect({
          to: targetPath,
          params: { realmId },
        })
      }

      // Realm root path (e.g., /admin, /admin/, /user, /user/): redirect authenticated users based on permissions
      if (isRealmRootPath && authenticated) {
        const hasAdmin = checkAdminPermission()
        const targetPath = hasAdmin ? '/$realmId/manage' : '/$realmId/user/profile'
        throw redirect({
          to: targetPath,
          params: { realmId },
        })
      }

      // Public routes (auth + legal): redirect authenticated users away from auth pages,
      // but allow them to stay on legal agreement pages.
      if (isAuthRoute && authenticated) {
        const targetPath = checkAdminPermission() ? '/$realmId/manage' : '/$realmId/user/profile'
        throw redirect({
          to: targetPath,
          params: { realmId },
        })
      }

      // Protected routes: require authentication (auth and legal are public)
      if (!isAuthRoute && !isLegalRoute && !authenticated) {
        // Extract the relative path (without realm prefix)
        const relativePath = pathname.replace(new RegExp(`^/${realmId}`), '') || '/'
        throw redirect({
          to: '/$realmId/auth/login',
          params: { realmId },
          search: { redirect: relativePath },
        })
      }

      // Manage route: require admin permission (only for authenticated users)
      if (isManageRoute && !checkAdminPermission()) {
        throw redirect({
          to: '/$realmId/user/profile',
          params: { realmId },
        })
      }

      // Return empty object (auth data is now in Zustand store)
      return {}
    } catch (error) {
      // If it's a redirect, re-throw it
      if (isRedirect(error)) {
        throw error
      }

      // Other errors: redirect to login (unless it's a public route)
      if (!isAuthRoute && !isLegalRoute) {
        const relativePath = pathname.replace(new RegExp(`^/${realmId}`), '') || '/'
        throw redirect({
          to: '/$realmId/auth/login',
          params: { realmId },
          search: { redirect: relativePath },
        })
      }

      // For auth routes, let them render (they handle their own errors)
      return {}
    }
  },

  component: RootComponent,
})

function RootComponent() {
  const router = useRouter()
  const queryClient = useQueryClient()
  const isAuthenticated = useIsAuthenticated()
  const pathname = router.state.location.pathname

  const pathSegments = pathname.split('/').filter(Boolean)
  const realmId = pathSegments[0] || 'admin'

  const isRootPath = pathname === '/'
  const isRealmRootPath = /^\/[^/]+\/?$/.test(pathname)
  const isAuthRoute = /^\/[^/]+\/auth\//.test(pathname)
  const isLegalRoute = /^\/[^/]+\/legal\//.test(pathname)

  const isCoreRoute =
    isAuthenticated && !isAuthRoute && !isLegalRoute && !isRootPath && !isRealmRootPath

  const { data: consentStatus } = useQuery({
    ...consentStatusQueryOptions(realmId),
    enabled: isCoreRoute,
  })

  const pendingItems = consentStatus?.items?.filter((item) => item.needs_reconsent) ?? []
  const needsReconsent = isCoreRoute && pendingItems.length > 0

  const consentMutation = useMutation({
    mutationFn: async () => {
      await recordConsentMutation(realmId, toRecordConsentRequestFromStatus(pendingItems))
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.consentStatus(realmId) })
    },
  })

  return (
    <>
      <div className="min-h-screen bg-background font-sans">
        <Outlet />
      </div>
      <Toaster />
      <Suspense>
        <Devtools />
      </Suspense>
      {needsReconsent && (
        <ReconsentDialog
          realmId={realmId}
          open={true}
          items={pendingItems}
          isPending={consentMutation.isPending}
          onAgree={() => consentMutation.mutate()}
          onLogout={() => logoutFlow(realmId)}
        />
      )}
    </>
  )
}
