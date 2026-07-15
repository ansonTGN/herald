import type { PublicConfigResponse } from '@/lib/api-generated'
import { useAuthStore } from '@/stores/auth-store'
import { useLocation, useRouter } from '@tanstack/react-router'

const CUSTOM_DOMAIN_ROOT_SEGMENTS = new Set(['auth', 'manage', 'user', 'legal', 'device'])
const SESSION_SCOPED_ROOT_SEGMENTS = new Set(['manage', 'user', 'subscription'])
const REALM_SCOPED_PUBLIC_ROOT_SEGMENTS = new Set(['auth', 'legal', 'device'])

export interface ResolvedRealmContext {
  realmId: string
  isCustomDomain: boolean
  publicConfig?: PublicConfigResponse
}

let cachedCustomDomainContext: ResolvedRealmContext | null = null

function firstPathSegment(pathname: string): string | null {
  return pathname.split('/').filter(Boolean)[0] ?? null
}

export function isCustomDomainPath(pathname: string): boolean {
  const first = firstPathSegment(pathname)
  return first === null || CUSTOM_DOMAIN_ROOT_SEGMENTS.has(first)
}

export function getLegacyRealmId(pathname: string): string {
  const first = firstPathSegment(pathname)
  if (first && SESSION_SCOPED_ROOT_SEGMENTS.has(first)) {
    return useAuthStore.getState().realmId || 'admin'
  }
  return first || 'admin'
}

export function getCachedCustomDomainRealm(): ResolvedRealmContext | null {
  return cachedCustomDomainContext
}

export async function resolveRealmContext(pathname: string): Promise<ResolvedRealmContext> {
  if (!isCustomDomainPath(pathname)) {
    return {
      realmId: getLegacyRealmId(pathname),
      isCustomDomain: false,
    }
  }

  if (cachedCustomDomainContext) {
    return cachedCustomDomainContext
  }

  const host = typeof window === 'undefined' ? '' : window.location.host
  const resolveUrl = host
    ? `/api/public-config/custom-domain/resolve?host=${encodeURIComponent(host)}`
    : '/api/public-config/custom-domain/resolve'

  const response = await fetch(resolveUrl, {
    credentials: 'include',
  })
  if (!response.ok) {
    // Main-domain session routes have no realm prefix. Their realm is loaded
    // from /api/auth/status by the root loader, not from custom-domain DNS.
    if (pathname === '/' || SESSION_SCOPED_ROOT_SEGMENTS.has(firstPathSegment(pathname) ?? '')) {
      return {
        realmId: getLegacyRealmId(pathname),
        isCustomDomain: false,
      }
    }
    throw new Error('Unable to resolve custom-domain realm')
  }

  const body = (await response.json()) as {
    data?: { realmId: string; publicConfig?: PublicConfigResponse }
    realmId?: string
    publicConfig?: PublicConfigResponse
  }
  const payload = body.data ?? body
  if (!payload.realmId) {
    throw new Error('Custom-domain resolve response did not include realmId')
  }

  cachedCustomDomainContext = {
    realmId: payload.realmId,
    isCustomDomain: true,
    publicConfig: payload.publicConfig,
  }
  return cachedCustomDomainContext
}

export function resolvedRealmFromPath(pathname: string): ResolvedRealmContext {
  if (isCustomDomainPath(pathname)) {
    const resolved = getCachedCustomDomainRealm()
    if (resolved) return resolved
  }

  return {
    realmId: getLegacyRealmId(pathname),
    isCustomDomain: false,
  }
}

function getWindowPathname(): string {
  return typeof window === 'undefined' ? '/' : window.location.pathname
}

function getWindowSearch(): Record<string, string> {
  if (typeof window === 'undefined') return {}
  return Object.fromEntries(new URLSearchParams(window.location.search))
}

export function useCurrentPathname(): string {
  try {
    return useLocation().pathname
  } catch {
    return getWindowPathname()
  }
}

export function useResolvedRealmContext(): ResolvedRealmContext {
  const pathname = useCurrentPathname()
  const sessionRealmId = useAuthStore((state) => state.realmId)
  const context = resolvedRealmFromPath(pathname)
  return SESSION_SCOPED_ROOT_SEGMENTS.has(firstPathSegment(pathname) ?? '')
    ? { ...context, realmId: sessionRealmId || context.realmId }
    : context
}

export function useResolvedRealmId(fallback: string = 'admin'): string {
  return useResolvedRealmContext().realmId || fallback
}

export function usePathSegments(): string[] {
  return useCurrentPathname().split('/').filter(Boolean)
}

export function useLastPathSegment(offsetFromEnd: number = 0): string {
  const segments = usePathSegments()
  return segments[segments.length - 1 - offsetFromEnd] ?? ''
}

export function useCurrentSearch<TSearch>(): TSearch {
  try {
    const router = useRouter()
    return router.state.location.search as TSearch
  } catch {
    return getWindowSearch() as TSearch
  }
}

/**
 * Read the params of a `createFileRoute` route without throwing when the
 * component is mounted under a different route tree.
 *
 * `$realmId/...` route components are reused by the prefix-less custom-domain
 * routes (e.g. `/auth/login` reuses `LoginPage`). On the custom-domain tree
 * there is no active match for `/$realmId/auth/login`, so `Route.useParams()`
 * throws. TanStack's own hooks dereference an undefined match when
 * `shouldThrow:false`, so the params cannot be recovered any other way; we
 * invoke `useParams` through the passed-in route object and catch, falling back
 * to an empty object so callers can use the resolved realm context. Calling it
 * indirectly (rather than as a static `Route.useParams()`) also keeps the
 * `react-hooks/rules-of-hooks` lint rule quiet.
 */
export function useOptionalRouteParams<TParams extends Record<string, string | undefined>>(route: {
  useParams: () => TParams
}): TParams {
  try {
    return route.useParams()
  } catch {
    return {} as TParams
  }
}

export function realmPath(context: ResolvedRealmContext, path: string): string {
  const normalized = path.startsWith('/') ? path : `/${path}`
  if (context.isCustomDomain) return normalized
  if (!REALM_SCOPED_PUBLIC_ROOT_SEGMENTS.has(firstPathSegment(normalized) ?? '')) {
    return normalized
  }
  return `/${context.realmId}${normalized}`
}
