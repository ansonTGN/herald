import { createRootRouteWithContext, Outlet, redirect, isRedirect } from '@tanstack/react-router'
import { ReactQueryDevtools } from '@tanstack/react-query-devtools'
import { TanStackRouterDevtools } from '@tanstack/router-devtools'
import { Toaster } from '@/components/ui/sonner'
import type { QueryClient } from '@tanstack/react-query'
import { initializeAuth, checkAdminPermission } from '@/lib/auth-utils'

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
    const isManageRoute = pathname.match(/^\/[^/]+\/manage/)

    try {
      // Initialize auth and populate Zustand store
      const result = await initializeAuth(realmId)
      const authenticated = result.authenticated

      console.log('[__root loader] Pathname:', pathname)
      console.log('[__root loader] Realm ID:', realmId)

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
        console.log(
          '[__root loader] Realm root path with authenticated user, checking permissions...'
        )
        const hasAdmin = checkAdminPermission()
        console.log('[__root loader] Has admin permission:', hasAdmin)
        const targetPath = hasAdmin ? '/$realmId/manage' : '/$realmId/user/profile'
        console.log('[__root loader] Redirecting to:', targetPath)
        throw redirect({
          to: targetPath,
          params: { realmId },
        })
      }

      // Auth route: redirect authenticated users to appropriate page
      if (isAuthRoute && authenticated) {
        const targetPath = checkAdminPermission() ? '/$realmId/manage' : '/$realmId/user/profile'
        throw redirect({
          to: targetPath,
          params: { realmId },
        })
      }

      // Protected routes: require authentication
      if (!isAuthRoute && !authenticated) {
        console.log('[__root loader] Protected route without authentication, redirecting to login')
        // Extract the relative path (without realm prefix)
        const relativePath = pathname.replace(new RegExp(`^/${realmId}`), '') || '/'
        console.log('[__root loader] Relative path:', relativePath)
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
      console.error('[__root loader] Error in loader:', error)

      // If it's a redirect, re-throw it
      if (isRedirect(error)) {
        console.log('[__root loader] Re-throwing redirect')
        throw error
      }

      // Other errors: redirect to login (unless it's an auth route)
      if (!isAuthRoute) {
        console.log('[__root loader] Error occurred, redirecting to login')
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
  return (
    <>
      <div className="min-h-screen bg-background font-sans">
        <Outlet />
      </div>
      <Toaster />
      <ReactQueryDevtools buttonPosition="bottom-right" />
      <TanStackRouterDevtools />
    </>
  )
}
