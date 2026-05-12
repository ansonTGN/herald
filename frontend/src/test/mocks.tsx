import { vi } from 'vitest'

/**
 * Mock for @marsidev/react-turnstile
 * Used in TurnstileWidget tests to simulate Cloudflare Turnstile widget.
 *
 * The mock stores callbacks on window._turnstileCallbacks so tests can
 * programmatically trigger success, error, and expire events.
 */
export function mockTurnstile() {
  vi.mock('@marsidev/react-turnstile', () => ({
    Turnstile: vi.fn(({ onSuccess, onError, onExpire }) => {
      // Store callbacks on window for test access
      if (typeof window !== 'undefined') {
        ;(window as any)._turnstileCallbacks = { onSuccess, onError, onExpire }
      }
      return <div data-testid="turnstile-mock" />
    }),
  }))
}

/**
 * Mock for @tanstack/react-router hooks
 * Used in AuthPageWrapper tests to provide router context.
 */
export function mockTanStackRouter() {
  vi.mock('@tanstack/react-router', () => ({
    useParams: vi.fn(() => ({ realmId: 'test-realm' })),
    useLocation: vi.fn(() => ({ pathname: '/test' })),
    Link: ({ children, to }: { children: React.ReactNode; to: string }) => (
      <a href={to}>{children}</a>
    ),
    useNavigate: vi.fn(() => vi.fn()),
  }))
}

/**
 * Type definition for window._turnstileCallbacks
 * Used to access Turnstile mock callbacks in tests.
 */
declare global {
  interface Window {
    _turnstileCallbacks?: {
      onSuccess?: (token: string) => void
      onError?: (error: string) => void
      onExpire?: () => void
    }
  }
}
