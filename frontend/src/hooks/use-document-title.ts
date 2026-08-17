/**
 * Document title management
 *
 * Sets `document.title` to `{pageName} · {realmName}` so that browser history
 * (e.g. Chrome's back-button dropdown) shows a meaningful, per-page title
 * instead of a single constant.
 *
 * The page name is derived from the URL's meaningful path segment. Sub-routes
 * that mutate an entity (edit/new/reveal/overview) fall back to their parent
 * segment so the title reflects the resource, not the action.
 *
 * Realm name comes from the public-config query; when missing it falls back to
 * the realmId, and when even that is empty it is omitted.
 */

import { useQuery } from '@tanstack/react-query'
import { useLocation } from '@tanstack/react-router'
import { useEffect } from 'react'
import { publicConfigQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'

/** Trailing segments that represent an action on a resource, not the resource itself. */
const ACTION_SEGMENTS = new Set(['edit', 'new', 'reveal', 'overview'])

/** Maps a meaningful URL segment to a translated label getter. */
const SEGMENT_LABELS: Record<string, () => string> = {
  // manage (admin)
  manage: m['nav.dashboard'],
  realms: m['nav.realms'],
  'client-apps': m['nav.clients'],
  users: m['nav.users'],
  permissions: m['nav.permissions'],
  roles: m['nav.roles'],
  'api-keys': m['nav.api_keys'],
  'payment-providers': m['nav.payment_providers'],
  'entitlement-mappings': m['nav.entitlement_mappings'],
  'registration-rules': m['nav.points_registration_rules'],
  'credit-buckets': m['nav.credit_buckets'],
  invoices: m['nav.invoices'],
  'subscription-history': m['nav.subscription_history'],
  wallets: m['nav.points_wallets'],
  audit: m['nav.audit_log'],
  settings: m['nav.settings'],
  // user (profile)
  profile: m['nav_profile.profile'],
  security: m['nav_profile.security'],
  points: m['nav_profile.points'],
  'purchase-points': m['nav_profile.purchase_records'],
  // shared
  subscription: m['nav_profile.subscription'],
}

/** Capitalize the first letter of a fallback segment for display. */
function capitalize(segment: string): string {
  return segment ? segment.charAt(0).toUpperCase() + segment.slice(1) : segment
}

/** True when a segment is an action sub-route (edit/new/...), not a resource. */
function isAction(segment: string): boolean {
  return ACTION_SEGMENTS.has(segment)
}

/**
 * Resolve a human-readable page name from a pathname.
 *
 * Strategy: walk the path from the end toward the root.
 *  1. Prefer the nearest non-action segment that has a known label — this is
 *     robust against route params of any shape (ids, slugs), since we key off
 *     the known-label set rather than guessing what "looks like an id".
 *  2. If no labeled ancestor exists, fall back to the last non-action segment
 *     that reads like a name (letters/dashes), capitalized.
 *  3. Otherwise ''.
 *
 * Returns '' for the bare root.
 */
export function resolvePageName(pathname: string): string {
  const segments = pathname.split('/').filter(Boolean)
  if (segments.length === 0) return ''

  let fallback = ''
  for (let i = segments.length - 1; i >= 0; i -= 1) {
    const seg = segments[i]
    if (isAction(seg)) continue
    const labelFn = SEGMENT_LABELS[seg]
    if (labelFn) return labelFn()
    if (!fallback && /^[a-z][a-z0-9-]*$/i.test(seg)) {
      fallback = capitalize(seg.replace(/-/g, ' '))
    }
  }
  return fallback
}

/**
 * Keep `document.title` in sync with the current route and realm name.
 *
 * @param realmId - The current realm id (used to fetch the realm display name).
 */
export function useDocumentTitle(realmId: string): void {
  const location = useLocation()
  const { data: publicConfig } = useQuery(publicConfigQueryOptions(realmId))

  useEffect(() => {
    const pageName = resolvePageName(location.pathname)
    const realmName = publicConfig?.realmName ?? realmId

    document.title = realmName ? `${pageName} · ${realmName}`.trim() : pageName
  }, [location.pathname, publicConfig?.realmName, realmId])
}
