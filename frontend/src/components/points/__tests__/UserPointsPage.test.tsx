/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import type { ListWalletsByBucketResponse, PointsTransactionResponse } from '@/lib/api-generated'
import { UserPointsPage } from '../UserPointsPage'

// Mock Link so the inline purchase CTA renders without a TanStack Router
// provider (matches the profile-sidebar test pattern).
vi.mock('@tanstack/react-router', () => ({
  Link: ({ children, ...props }: { children: ReactNode }) => <a {...props}>{children}</a>,
}))

// Mock the query-options module so we can seed the wallets + transactions
// responses without standing up MSW routes. This locks the user-view contract:
// bucket names come from the wallets response (NOT the admin-only credit-buckets
// directory, which 403s for regular users — see Bug B).
vi.mock('@/data/query-options', () => ({
  walletsByBucketQueryOptions: vi.fn(),
  pointsTransactionsQueryOptions: vi.fn(),
  featureAvailabilityQueryOptions: vi.fn(),
}))

import {
  walletsByBucketQueryOptions,
  pointsTransactionsQueryOptions,
  featureAvailabilityQueryOptions,
} from '@/data/query-options'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  })
}

const REALM_ID = 'realm-001'
const USER_ID = 'user-self'

const walletsResponse: ListWalletsByBucketResponse = {
  crossBucketTotal: 100,
  items: [
    {
      bucketId: 'bucket-primary',
      name: 'Primary Pool',
      enabled: true,
      bucketTotal: 100,
      userId: USER_ID,
      balancesByType: {
        freePeriodic: 0,
        granted: 0,
        registration: 0,
        subscription: 0,
        topup: 100,
      },
    },
    {
      bucketId: 'bucket-promo',
      name: 'Promo Pool',
      enabled: true,
      bucketTotal: 0,
      userId: USER_ID,
      balancesByType: {
        freePeriodic: 0,
        granted: 0,
        registration: 0,
        subscription: 0,
        topup: 0,
      },
    },
  ],
}

const txnWithBucket: PointsTransactionResponse = {
  id: 'txn-1',
  walletId: 'wallet-1',
  userId: USER_ID,
  realmId: REALM_ID,
  amount: 100,
  balanceAfter: 100,
  transactionType: 'recharge',
  description: 'Top up',
  externalRefId: 'ref-1',
  bucketId: 'bucket-primary',
  createdAt: '2025-03-15T10:00:00Z',
}

const transactionsResponse = { transactions: [txnWithBucket] }

function mockQueryOptions() {
  vi.mocked(walletsByBucketQueryOptions).mockReturnValue({
    queryKey: ['wallets', REALM_ID],
    queryFn: async () => walletsResponse,
  } as never)
  vi.mocked(pointsTransactionsQueryOptions).mockReturnValue({
    queryKey: ['txns', REALM_ID],
    queryFn: async () => transactionsResponse,
  } as never)
  vi.mocked(featureAvailabilityQueryOptions).mockReturnValue({
    queryKey: ['feature-availability', REALM_ID],
    queryFn: async () => ({ user: { pointsPurchaseVisible: false } }),
  } as never)
}

function renderPage(overrides: { queryClient?: QueryClient } = {}) {
  const qc = overrides.queryClient ?? createTestQueryClient()
  return render(
    <QueryClientProvider client={qc}>
      <UserPointsPage
        realmId={REALM_ID}
        userId={USER_ID}
        bucketId={undefined}
        onBucketIdChange={vi.fn()}
      />
    </QueryClientProvider>
  )
}

describe('UserPointsPage bucket option derivation', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockQueryOptions()
  })

  it('GIVEN regular user wallets WHEN rendering THEN bucket Select lists wallet-derived names (not admin directory)', async () => {
    const user = userEvent.setup({ delay: null })
    renderPage()

    // Wait for wallets query to settle so the bucket Select renders.
    await waitFor(() => expect(screen.getByTestId('transaction-history-table')).toBeInTheDocument())

    const bucketSelect = screen.getByRole('combobox', { name: /bucket/i })
    await user.click(bucketSelect)

    // Both wallet-derived names must be selectable. The admin credit-buckets
    // directory is NOT called for the user view; names come from wallets.
    expect(screen.getByRole('option', { name: 'Primary Pool' })).toBeInTheDocument()
    expect(screen.getByRole('option', { name: 'Promo Pool' })).toBeInTheDocument()
  })

  it('GIVEN a transaction whose bucketId matches a held wallet WHEN rendering THEN shows the bucket NAME in the transaction-bucket cell', async () => {
    renderPage()

    const bucketCell = await screen.findByTestId('transaction-bucket-0')
    expect(bucketCell).toHaveTextContent('Primary Pool')
  })
})

describe('UserPointsPage inline purchase block', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockQueryOptions()
  })

  it('GIVEN pointsPurchaseVisible=true WHEN rendering THEN shows the inline purchase CTA', async () => {
    vi.mocked(featureAvailabilityQueryOptions).mockReturnValue({
      queryKey: ['feature-availability', REALM_ID],
      queryFn: async () => ({ user: { pointsPurchaseVisible: true } }),
    } as never)

    renderPage()

    await waitFor(() =>
      expect(screen.getByTestId('points-purchase-inline-block')).toBeInTheDocument()
    )
    expect(screen.getByTestId('points-purchase-cta')).toBeInTheDocument()
  })

  it('GIVEN pointsPurchaseVisible=false WHEN rendering THEN hides the inline purchase block', async () => {
    renderPage()

    await waitFor(() => expect(screen.getByTestId('transaction-history-table')).toBeInTheDocument())
    expect(screen.queryByTestId('points-purchase-inline-block')).not.toBeInTheDocument()
    expect(screen.queryByTestId('points-purchase-cta')).not.toBeInTheDocument()
  })
})
