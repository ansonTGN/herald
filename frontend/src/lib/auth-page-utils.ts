/**
 * @deprecated Use @/lib/auth-guards instead
 * This module is kept for backward compatibility
 */
import { redirectIfAuthenticated as _redirectIfAuthenticated } from './auth-guards'

export const AUTH_QUERY_KEY = (realmId: string) => ['auth', realmId, 'status'] as const

export function getRedirectPath(realmId: string, redirect?: string | string[]): string {
  if (redirect) {
    return String(redirect)
  }
  return `/${realmId}`
}

// Re-export from new module for backward compatibility
export const redirectIfAuthenticated = (realmId: string, redirectPath?: string) => {
  return _redirectIfAuthenticated(realmId, redirectPath)
}
