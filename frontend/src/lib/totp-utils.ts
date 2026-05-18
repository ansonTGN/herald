import { toast } from 'sonner'
import { useRouter } from '@tanstack/react-router'
import { useQueryClient } from '@tanstack/react-query'

const AUTH_QUERY_KEY = (realmId: string) => ['auth', realmId, 'status'] as const
const REQUEST_TIMEOUT_MS = 30000

export interface TotpData {
  secret: string
  qrCodeUrl: string
  backupCodes: string[]
  tempToken: string
}

/**
 * Wraps a promise with a timeout to prevent hanging requests
 */
export function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number = REQUEST_TIMEOUT_MS
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error('Request timeout')), timeoutMs)
    ),
  ])
}

/**
 * Common handler for successful TOTP verification during login
 */
export function handleTotpLoginSuccess(
  queryClient: ReturnType<typeof useQueryClient>,
  router: ReturnType<typeof useRouter>,
  realmId: string,
  redirectPath?: string | null
) {
  queryClient.invalidateQueries({ queryKey: AUTH_QUERY_KEY(realmId) })
  queryClient.refetchQueries({ queryKey: AUTH_QUERY_KEY(realmId) })
  toast.success('Login successful')
  const safeRedirectTo = String(redirectPath || `/${realmId}`)
  router.navigate({ to: safeRedirectTo })
}

/**
 * Formats a date for display
 */
export function formatDate(dateString: string | null | undefined): string {
  if (!dateString) return 'Unknown'
  return new Date(dateString).toLocaleString()
}
