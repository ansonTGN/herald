import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { EntitlementMappingResponse } from '@/lib/api-generated'

vi.mock('@/hooks/use-permission', () => ({
  usePermission: () => ({ hasPermission: () => true }),
}))

vi.mock('@/data/query-options', () => ({
  queryKeys: {
    entitlementMappings: (realmId: string) => ['entitlement-mappings', realmId, {}],
  },
  entitlementMappingsQueryOptions: (realmId: string) => ({
    queryKey: ['entitlement-mappings', realmId, {}],
    queryFn: async () => ({ items: [], total: 0 }),
    staleTime: Number.POSITIVE_INFINITY,
  }),
  adminRolesQueryOptions: (realmId: string) => ({
    queryKey: ['roles', realmId],
    queryFn: async () => [],
  }),
  creditBucketsListQueryOptions: (realmId: string) => ({
    queryKey: ['credit-buckets', realmId],
    queryFn: async () => [
      {
        id: 'bucket-a',
        name: 'General',
        bucketKey: 'general',
        displayOrder: 0,
        enabled: true,
        coveredClientAppCount: 1,
        ruleReferenceCount: 1,
      },
      {
        id: 'bucket-b',
        name: 'Images',
        bucketKey: 'images',
        displayOrder: 1,
        enabled: true,
        coveredClientAppCount: 1,
        ruleReferenceCount: 1,
      },
    ],
  }),
}))

const { batchMutate } = vi.hoisted(() => ({ batchMutate: vi.fn() }))

vi.mock('@/data/entitlement-mapping-mutations', () => ({
  useBatchUpdateEntitlementMappings: () => ({ mutate: batchMutate, isPending: false }),
  useUpdateEntitlementMapping: () => ({ mutate: vi.fn(), isPending: false }),
  useCreateEntitlementMapping: () => ({ mutate: vi.fn(), isPending: false }),
  isProtectedPriceError: () => false,
  extractActiveSubscriptions: () => null,
}))

vi.mock('@/components/billing/provider-sync-button', () => ({
  ProviderSyncButton: () => <div data-testid="provider-sync-button" />,
}))

vi.mock('@/components/shared/role-selector', () => ({
  RoleSelector: () => <div data-testid="role-selector" />,
}))

import { EntitlementMappingsPage } from '../entitlement-mappings-page'

const mapping: EntitlementMappingResponse = {
  id: 'mapping-1',
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  paymentProvider: 'stripe',
  externalProductId: 'prod-1',
  externalPriceId: 'price-1',
  entitlementKey: 'pro',
  enabled: true,
  billingType: 'recurring',
  billingPeriod: 'month',
  grantedRoleIds: [],
  pointRules: [
    {
      id: 'rule-1',
      bucketId: 'bucket-a',
      triggerSources: ['subscription_initial', 'subscription_renewal'],
      grantMode: 'fixed',
      pointsAmount: 100,
      validityDays: 30,
      enabled: true,
      displayOrder: 0,
    },
    {
      id: 'rule-2',
      bucketId: 'bucket-b',
      triggerSources: ['subscription_initial'],
      grantMode: 'quota',
      quotaWindows: [{ key: 'hour', windowSeconds: 3600, limit: 20 }],
      enabled: true,
      displayOrder: 1,
    },
  ],
}

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  client.setQueryData(['entitlement-mappings', 'realm-1', {}], {
    items: [mapping],
    total: 1,
  })
  return render(
    <QueryClientProvider client={client}>
      <EntitlementMappingsPage realmId="realm-1" />
    </QueryClientProvider>
  )
}

describe('EntitlementMappingsPage distribution rules', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders each fixed and quota rule without collapsing them into one amount', async () => {
    renderPage()
    expect(await screen.findByTestId('point-rule-rule-1')).toBeInTheDocument()
    expect(screen.getByTestId('point-rule-rule-2')).toBeInTheDocument()
    expect(screen.getAllByTestId('point-rule-trigger-subscription_initial')).toHaveLength(2)
    expect(screen.getByTestId('point-rule-quota-rule-2-editor')).toBeInTheDocument()
  })

  it('turns removal of a persisted rule into an explicit disabled upsert', async () => {
    const user = userEvent.setup()
    renderPage()

    await user.click(await screen.findByTestId('point-rule-remove-rule-1'))
    await user.click(screen.getByTestId('save-mapping-button'))

    expect(batchMutate).toHaveBeenCalledOnce()
    const request = batchMutate.mock.calls[0]?.[0]
    expect(request.updates[0].pointRules).toEqual(
      expect.arrayContaining([expect.objectContaining({ id: 'rule-1', enabled: false })])
    )
    expect(request.updates[0]).not.toHaveProperty('pointsPerPeriod')
    expect(request.updates[0]).not.toHaveProperty('bucketId')
  })
})
