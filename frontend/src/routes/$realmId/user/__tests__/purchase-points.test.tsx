import { describe, it, expect, vi, beforeAll, beforeEach, afterEach } from 'vitest'
import { render, act, screen } from '@testing-library/react'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { server } from '@/test/mocks/server'
import { createTestQueryClient } from '@/test/utils/render'

// --- Mock router ---

const mockNavigate = vi.fn()
const realmIdParam = 'test-realm'

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = (await importOriginal()) as Record<string, any>
  return {
    ...actual,
    createFileRoute: () => (opts: any) => ({
      ...opts,
      useParams: () => ({ realmId: realmIdParam }),
    }),
    useNavigate: () => mockNavigate,
  }
})

// --- Mock stores ---

let mockCanRecoverReturn = false
let mockPaymentAttemptReturn: {
  attemptId: string | null
  attemptStatus: string | null
  paymentContext: Record<string, any> | null
  expiresAt: string | null
} = {
  attemptId: null,
  attemptStatus: null,
  paymentContext: null,
  expiresAt: null,
}

let mockPurchaseStateReturn: {
  realmId: string | null
  userId: string | null
  targetType: string | null
  targetId: string | null
  paymentProvider: string | null
} = {
  realmId: null,
  userId: null,
  targetType: null,
  targetId: null,
  paymentProvider: null,
}

vi.mock('@/stores/purchase-flow-store', () => ({
  usePurchaseFlowActions: () => ({
    setPurchaseState: vi.fn(),
    setPaymentAttempt: vi.fn(),
    clearPurchaseState: vi.fn(),
    canRecover: () => mockCanRecoverReturn,
  }),
  usePaymentAttempt: () => mockPaymentAttemptReturn,
  usePurchaseState: () => mockPurchaseStateReturn,
  usePurchaseFlowStore: (selector: any) => selector(mockPurchaseStateReturn),
}))

vi.mock('@/stores/auth-store', () => ({
  useAuthStore: () => ({
    user: { id: 'user-1' },
  }),
}))

// --- Mock window.location.href ---

const hrefSetter = vi.fn()
const originalLocation = window.location

function mockWindowLocation() {
  const mockLoc = {
    ...originalLocation,
    get href() {
      return ''
    },
    set href(val: string) {
      hrefSetter(val)
    },
  }
  Object.defineProperty(window, 'location', {
    value: mockLoc,
    writable: true,
    configurable: true,
  })
}

function restoreWindowLocation() {
  Object.defineProperty(window, 'location', {
    value: originalLocation,
    writable: true,
    configurable: true,
  })
}

// --- Helpers ---

function makePaymentContextForPage(overrides?: Partial<Record<string, any>>): Record<string, any> {
  return {
    paymentProvider: 'stripe',
    stripeCheckoutUrl: null,
    creemCheckoutUrl: null,
    wechatCodeUrl: null,
    clientSecret: null,
    ...overrides,
  }
}

const FUTURE_EXPIRES = new Date(Date.now() + 3600 * 1000).toISOString()

/**
 * Set up store mocks so the recovery useEffect fires and sets
 * currentStep to 'processing'. This triggers the auto-redirect effect.
 */
function configureRecoveryState(
  paymentContext: Record<string, any> | null,
  attemptId: string = 'attempt-1'
) {
  mockCanRecoverReturn = true
  mockPaymentAttemptReturn = {
    attemptId,
    attemptStatus: 'Pending',
    paymentContext,
    expiresAt: FUTURE_EXPIRES,
  }
  mockPurchaseStateReturn = {
    realmId: 'test-realm',
    userId: 'user-1',
    targetType: 'points_package',
    targetId: 'pkg-1',
    paymentProvider: paymentContext?.paymentProvider ?? 'stripe',
  }
}

/**
 * Provide MSW handlers for ALL queries the page might make.
 */
function installPageQueryHandlers() {
  server.use(
    http.get('*/api/bill/:realmId/points-packages', () => HttpResponse.json({ packages: [] })),
    http.get('*/api/bill/:realmId/payment-providers', () => HttpResponse.json({ providers: [] })),
    http.get('*/api/third/pay/:realmId/providers', () => HttpResponse.json({ providers: [] })),
    http.get('*/api/bill/:realmId/purchase/payment-attempts/:attemptId', () =>
      HttpResponse.json({
        id: 'attempt-1',
        status: 'Pending',
        targetType: 'points_package',
        targetId: 'pkg-1',
        amount: 9.99,
        currency: 'USD',
        createdAt: new Date().toISOString(),
        expiresAt: FUTURE_EXPIRES,
        completedAt: null,
        fulfillment: null,
        providerStatus: null,
      })
    )
  )
}

// --- Import page under test (after mocks are hoisted) ---
import { Route } from '../purchase-points'

/**
 * Render the purchase-points page component and wait for it to mount.
 *
 * The component triggers React Query fetches on mount which cause
 * React to suspend. Wrapping render in act(async ...) and giving
 * the event loop time to settle lets MSW responses arrive before
 * we try to assert on the DOM.
 */
async function renderPage() {
  const queryClient = createTestQueryClient()
  installPageQueryHandlers()
  const PageComponent = Route.component as React.ComponentType

  // Render inside act and give the event loop time to settle so that
  // React Query + MSW can resolve before we try to assert on the DOM.
  let result: ReturnType<typeof render>
  await act(async () => {
    result = render(
      <QueryClientProvider client={queryClient}>
        <PageComponent />
      </QueryClientProvider>
    )
    // Give MSW + React Query time to resolve the fetch promises.
    // Without this, the first render is blank because queries are still loading.
    await new Promise((resolve) => setTimeout(resolve, 800))
  })

  return result!
}

// --- Tests ---

describe('Purchase-points auto-redirect state machine', () => {
  // Warm-up: do a single render before all tests to initialize React Query
  // and MSW. The first render triggers Suspense because React Query hasn't
  // cached data yet. Subsequent renders work fine because the test infrastructure
  // is already warmed up.
  beforeAll(async () => {
    mockWindowLocation()
    const queryClient = createTestQueryClient()
    installPageQueryHandlers()
    const PageComponent = Route.component as React.ComponentType
    await act(async () => {
      const { unmount } = render(
        <QueryClientProvider client={queryClient}>
          <PageComponent />
        </QueryClientProvider>
      )
      await new Promise((resolve) => setTimeout(resolve, 200))
      unmount()
    })
    restoreWindowLocation()
  })

  beforeEach(() => {
    mockWindowLocation()
    mockNavigate.mockClear()
    hrefSetter.mockClear()

    // Reset store mock state
    mockCanRecoverReturn = false
    mockPaymentAttemptReturn = {
      attemptId: null,
      attemptStatus: null,
      paymentContext: null,
      expiresAt: null,
    }
    mockPurchaseStateReturn = {
      realmId: null,
      userId: null,
      targetType: null,
      targetId: null,
      paymentProvider: null,
    }
  })

  afterEach(() => {
    restoreWindowLocation()
  })

  describe('Stripe checkout redirect', () => {
    it('redirects to stripeCheckoutUrl after 3-second timer', async () => {
      const checkoutUrl = 'https://checkout.stripe.com/pay/cs_test_123'
      configureRecoveryState(
        makePaymentContextForPage({
          paymentProvider: 'stripe',
          stripeCheckoutUrl: checkoutUrl,
        })
      )

      await renderPage()

      // Wait for the recovery useEffect to set currentStep='processing'
      // and the auto-redirect useEffect to schedule the 3s setTimeout.
      // Use a long timeout because the first render may take time to resolve
      // React Query fetches via MSW.
      await screen.findByTestId('purchase-step-processing', undefined, { timeout: 3000 })

      // Before 3s, no redirect should happen.
      await new Promise((resolve) => setTimeout(resolve, 2900))
      expect(hrefSetter).not.toHaveBeenCalled()

      // After 3s, the redirect fires.
      await new Promise((resolve) => setTimeout(resolve, 200))
      expect(hrefSetter).toHaveBeenCalledWith(checkoutUrl)
      expect(hrefSetter).toHaveBeenCalledTimes(1)
    }, 10000)
  })

  describe('Creem checkout redirect', () => {
    it('redirects to creemCheckoutUrl after 3-second timer', async () => {
      const checkoutUrl = 'https://checkout.creem.io/pay/test_456'
      configureRecoveryState(
        makePaymentContextForPage({
          paymentProvider: 'creem',
          creemCheckoutUrl: checkoutUrl,
        })
      )

      await renderPage()
      await screen.findByTestId('purchase-step-processing', undefined, { timeout: 3000 })

      // Wait just past 3s for the redirect timer to fire.
      await new Promise((resolve) => setTimeout(resolve, 3200))
      expect(hrefSetter).toHaveBeenCalledWith(checkoutUrl)
      expect(hrefSetter).toHaveBeenCalledTimes(1)
    }, 10000)
  })

  describe('WeChat (no redirect)', () => {
    it('does not redirect when only wechatCodeUrl is present', async () => {
      configureRecoveryState(
        makePaymentContextForPage({
          paymentProvider: 'wechat',
          wechatCodeUrl: 'weixin://wxpay/test',
        })
      )

      await renderPage()
      await screen.findByTestId('purchase-step-processing', undefined, { timeout: 3000 })

      // Wait well past the 3s mark -- no redirect should happen for WeChat.
      await new Promise((resolve) => setTimeout(resolve, 5000))
      expect(hrefSetter).not.toHaveBeenCalled()
    }, 10000)
  })

  describe('Cleanup cancels timeout on unmount', () => {
    it('does not redirect if component unmounts before timer fires', async () => {
      const checkoutUrl = 'https://checkout.stripe.com/pay/cs_cleanup'
      configureRecoveryState(
        makePaymentContextForPage({
          paymentProvider: 'stripe',
          stripeCheckoutUrl: checkoutUrl,
        })
      )

      const { unmount } = await renderPage()
      await screen.findByTestId('purchase-step-processing', undefined, { timeout: 3000 })

      // Wait 1 second (less than 3s), then unmount.
      await new Promise((resolve) => setTimeout(resolve, 1000))
      unmount()

      // Advance well past the original 3s mark -- cleanup should have cancelled the timer.
      await new Promise((resolve) => setTimeout(resolve, 5000))
      expect(hrefSetter).not.toHaveBeenCalled()
    }, 10000)
  })

  describe('No redirect when not in processing step', () => {
    it('does not redirect when canRecover is false (page stays at packages step)', async () => {
      // canRecover=false means the recovery useEffect won't set currentStep to 'processing'.
      // The page starts at 'packages', so the auto-redirect effect returns early.
      mockCanRecoverReturn = false
      mockPaymentAttemptReturn = {
        attemptId: 'attempt-1',
        attemptStatus: 'Pending',
        paymentContext: makePaymentContextForPage({
          paymentProvider: 'stripe',
          stripeCheckoutUrl: 'https://checkout.stripe.com/pay/cs_no_redirect',
        }),
        expiresAt: FUTURE_EXPIRES,
      }

      await renderPage()

      // Should be at packages step, not processing.
      await screen.findByTestId('purchase-step-packages', undefined, { timeout: 3000 })

      // Wait well past any potential timer.
      await new Promise((resolve) => setTimeout(resolve, 5000))
      expect(hrefSetter).not.toHaveBeenCalled()
    }, 10000)
  })
})
