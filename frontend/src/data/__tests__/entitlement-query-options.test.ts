import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  queryKeys,
  entitlementMappingsQueryOptions,
  entitlementMappingQueryOptions,
  subscriptionsQueryOptions,
  subscriptionDetailQueryOptions,
} from '@/data/query-options'
import { QUERY_KEYS } from '@/lib/constants'

// Mock client for list queries that use client.get() directly
vi.mock('@/lib/api-generated/client.gen', () => {
  const get = vi.fn()
  return { client: { get } }
})

// Mock SDK functions for detail queries
vi.mock('@/lib/api-generated/sdk.gen', async (importOriginal) => {
  const original = await importOriginal<typeof import('@/lib/api-generated/sdk.gen')>()
  return {
    ...original,
    getEntitlementMapping: vi.fn(),
    getSubscription: vi.fn(),
  }
})

import { client } from '@/lib/api-generated/client.gen'
import { getEntitlementMapping, getSubscription } from '@/lib/api-generated/sdk.gen'

describe('entitlement mapping query keys', () => {
  describe('filter isolation', () => {
    it('differentiates empty filters from paymentProvider filter', () => {
      const keyEmpty = queryKeys.entitlementMappings('realm-1', {})
      const keyFiltered = queryKeys.entitlementMappings('realm-1', { paymentProvider: 'stripe' })
      expect(keyEmpty).not.toEqual(keyFiltered)
    })

    it('differentiates different paymentProvider values', () => {
      const keyStripe = queryKeys.entitlementMappings('realm-1', { paymentProvider: 'stripe' })
      const keyCreem = queryKeys.entitlementMappings('realm-1', { paymentProvider: 'creem' })
      expect(keyStripe).not.toEqual(keyCreem)
    })

    it('differentiates enabled true vs false', () => {
      const keyEnabled = queryKeys.entitlementMappings('realm-1', { enabled: true })
      const keyDisabled = queryKeys.entitlementMappings('realm-1', { enabled: false })
      expect(keyEnabled).not.toEqual(keyDisabled)
    })

    it('differentiates combined filters from single filter', () => {
      const keyCombined = queryKeys.entitlementMappings('realm-1', {
        paymentProvider: 'stripe',
        enabled: true,
      })
      const keySingle = queryKeys.entitlementMappings('realm-1', { paymentProvider: 'stripe' })
      expect(keyCombined).not.toEqual(keySingle)
    })

    it('differentiates combined filters with different providers', () => {
      const keyStripe = queryKeys.entitlementMappings('realm-1', {
        paymentProvider: 'stripe',
        enabled: true,
      })
      const keyCreem = queryKeys.entitlementMappings('realm-1', {
        paymentProvider: 'creem',
        enabled: true,
      })
      expect(keyStripe).not.toEqual(keyCreem)
    })
  })

  describe('realm isolation', () => {
    it('differentiates same filters across different realms', () => {
      const keyRealm1 = queryKeys.entitlementMappings('realm-1', {})
      const keyRealm2 = queryKeys.entitlementMappings('realm-2', {})
      expect(keyRealm1).not.toEqual(keyRealm2)
    })
  })

  describe('detail isolation', () => {
    it('differentiates different mapping IDs', () => {
      const keyMapping1 = queryKeys.entitlementMapping('realm-1', 'mapping-1')
      const keyMapping2 = queryKeys.entitlementMapping('realm-1', 'mapping-2')
      expect(keyMapping1).not.toEqual(keyMapping2)
    })
  })

  describe('key structure', () => {
    it('list key starts with correct prefix', () => {
      const key = queryKeys.entitlementMappings('realm-1', {})
      expect(key[0]).toBe(QUERY_KEYS.ENTITLEMENT_MAPPINGS)
      expect(key[1]).toBe('realm-1')
    })

    it('detail key has correct structure', () => {
      const key = queryKeys.entitlementMapping('realm-1', 'mapping-1')
      expect(key).toEqual([QUERY_KEYS.ENTITLEMENT_MAPPING, 'realm-1', 'mapping-1'])
    })
  })
})

describe('subscription query keys', () => {
  describe('entitlementKey filter isolation', () => {
    it('differentiates empty filters from entitlementKey filter', () => {
      const keyEmpty = queryKeys.subscriptions('realm-1', {})
      const keyFiltered = queryKeys.subscriptions('realm-1', { entitlementKey: 'pro-plan' })
      expect(keyEmpty).not.toEqual(keyFiltered)
    })

    it('differentiates different entitlementKey values', () => {
      const keyPro = queryKeys.subscriptions('realm-1', { entitlementKey: 'pro' })
      const keyBasic = queryKeys.subscriptions('realm-1', { entitlementKey: 'basic' })
      expect(keyPro).not.toEqual(keyBasic)
    })
  })

  describe('status filter isolation', () => {
    it('differentiates different status values', () => {
      const keyActive = queryKeys.subscriptions('realm-1', { status: 'active' })
      const keyCanceled = queryKeys.subscriptions('realm-1', { status: 'canceled' })
      expect(keyActive).not.toEqual(keyCanceled)
    })
  })

  describe('detail isolation', () => {
    it('differentiates different subscription IDs', () => {
      const keySub1 = queryKeys.adminSubscription('realm-1', 'sub-1')
      const keySub2 = queryKeys.adminSubscription('realm-1', 'sub-2')
      expect(keySub1).not.toEqual(keySub2)
    })
  })

  describe('key structure', () => {
    it('list key starts with correct prefix', () => {
      const key = queryKeys.subscriptions('realm-1', {})
      expect(key[0]).toBe(QUERY_KEYS.ADMIN_SUBSCRIPTIONS)
      expect(key[1]).toBe('realm-1')
    })

    it('detail key has correct structure', () => {
      const key = queryKeys.adminSubscription('realm-1', 'sub-1')
      expect(key).toEqual([QUERY_KEYS.ADMIN_SUBSCRIPTION, 'realm-1', 'sub-1'])
    })
  })
})

describe('subscription query options - constructs correct request via client.get', () => {
  beforeEach(() => {
    vi.mocked(client.get).mockResolvedValue({
      data: { items: [], total: 0 },
      error: undefined,
    })
  })

  it('passes camelCase query params to client.get', async () => {
    const options = subscriptionsQueryOptions('realm-1', {
      entitlementKey: 'pro-plan',
      status: 'active',
      paymentProvider: 'stripe',
      page: 2,
      pageSize: 50,
    })

    await options.queryFn()

    expect(client.get).toHaveBeenCalledTimes(1)
    const callArgs = vi.mocked(client.get).mock.calls[0][0]

    expect(callArgs.url).toBe('/api/bill/realm-1/subscriptions')
    expect(callArgs.query).toEqual({
      entitlementKey: 'pro-plan',
      status: 'active',
      paymentProvider: 'stripe',
      page: 2,
      pageSize: 50,
    })

    // Verify no snake_case params leak
    const queryKeys = Object.keys(callArgs.query as Record<string, unknown>)
    for (const key of queryKeys) {
      expect(key).not.toMatch(/_/)
    }
  })

  it('omits undefined filter params', async () => {
    const options = subscriptionsQueryOptions('realm-1', {
      entitlementKey: 'pro-plan',
    })

    await options.queryFn()

    const callArgs = vi.mocked(client.get).mock.calls[0][0]
    expect(callArgs.query).toEqual({
      entitlementKey: 'pro-plan',
    })
  })

  it('throws when response has error', async () => {
    vi.mocked(client.get).mockResolvedValue({
      data: undefined,
      error: { message: 'Server error', status: 500 },
    })

    const options = subscriptionsQueryOptions('realm-1', {})

    await expect(options.queryFn()).rejects.toEqual({
      message: 'Server error',
      status: 500,
    })
  })
})

describe('entitlement mapping query options - constructs correct request via client.get', () => {
  beforeEach(() => {
    vi.mocked(client.get).mockResolvedValue({
      data: { items: [], total: 0 },
      error: undefined,
    })
  })

  it('passes camelCase query params to client.get', async () => {
    const options = entitlementMappingsQueryOptions('realm-1', {
      paymentProvider: 'stripe',
      enabled: true,
      page: 1,
      pageSize: 25,
    })

    await options.queryFn()

    expect(client.get).toHaveBeenCalledTimes(1)
    const callArgs = vi.mocked(client.get).mock.calls[0][0]

    expect(callArgs.url).toBe('/api/bill/realm-1/entitlement-mappings')
    expect(callArgs.query).toEqual({
      paymentProvider: 'stripe',
      enabled: true,
      page: 1,
      pageSize: 25,
    })

    // Verify no snake_case params leak
    const queryKeys = Object.keys(callArgs.query as Record<string, unknown>)
    for (const key of queryKeys) {
      expect(key).not.toMatch(/_/)
    }
  })

  it('omits undefined filter params', async () => {
    const options = entitlementMappingsQueryOptions('realm-1', {
      enabled: false,
    })

    await options.queryFn()

    const callArgs = vi.mocked(client.get).mock.calls[0][0]
    expect(callArgs.query).toEqual({ enabled: false })
  })

  it('throws when response has error', async () => {
    vi.mocked(client.get).mockResolvedValue({
      data: undefined,
      error: { message: 'Not found', status: 404 },
    })

    const options = entitlementMappingsQueryOptions('realm-1', {})

    await expect(options.queryFn()).rejects.toEqual({
      message: 'Not found',
      status: 404,
    })
  })
})

describe('entitlement mapping detail query option - uses generated SDK', () => {
  const mockMappingResponse = {
    id: 'mapping-1',
    entitlementKey: 'pro-plan',
    enabled: true,
    externalProductId: 'prod-123',
    grantOnSubscribe: false,
    createdAt: '2024-01-01T00:00:00Z',
  }

  beforeEach(() => {
    vi.mocked(getEntitlementMapping).mockResolvedValue({
      data: mockMappingResponse,
      error: undefined,
    })
  })

  it('calls getEntitlementMapping with correct path params', async () => {
    const options = entitlementMappingQueryOptions('realm-1', 'mapping-1')

    await options.queryFn()

    expect(getEntitlementMapping).toHaveBeenCalledTimes(1)
    expect(getEntitlementMapping).toHaveBeenCalledWith({
      path: { realmId: 'realm-1', mappingId: 'mapping-1' },
    })
  })

  it('resolves with mapping detail data', async () => {
    const options = entitlementMappingQueryOptions('realm-1', 'mapping-1')

    const result = await options.queryFn()

    expect(result).toEqual(mockMappingResponse)
  })

  it('throws when response has error', async () => {
    vi.mocked(getEntitlementMapping).mockResolvedValue({
      data: undefined,
      error: { message: 'Not found', status: 404 },
    })

    const options = entitlementMappingQueryOptions('realm-1', 'mapping-1')

    await expect(options.queryFn()).rejects.toEqual({
      message: 'Not found',
      status: 404,
    })
  })
})

describe('subscription detail query option - uses generated SDK', () => {
  const mockSubscriptionResponse = {
    id: 'sub-1',
    entitlementKey: 'pro-plan',
    paymentProvider: 'stripe',
    createdAt: '2024-01-01T00:00:00Z',
  }

  beforeEach(() => {
    vi.mocked(getSubscription).mockResolvedValue({
      data: mockSubscriptionResponse,
      error: undefined,
    })
  })

  it('calls getSubscription with correct path params', async () => {
    const options = subscriptionDetailQueryOptions('realm-1', 'sub-1')

    await options.queryFn()

    expect(getSubscription).toHaveBeenCalledTimes(1)
    expect(getSubscription).toHaveBeenCalledWith({
      path: { realmId: 'realm-1', subscriptionId: 'sub-1' },
    })
  })

  it('resolves with subscription detail data', async () => {
    const options = subscriptionDetailQueryOptions('realm-1', 'sub-1')

    const result = await options.queryFn()

    expect(result).toEqual(mockSubscriptionResponse)
  })

  it('throws when response has error', async () => {
    vi.mocked(getSubscription).mockResolvedValue({
      data: undefined,
      error: { message: 'Not found', status: 404 },
    })

    const options = subscriptionDetailQueryOptions('realm-1', 'sub-1')

    await expect(options.queryFn()).rejects.toEqual({
      message: 'Not found',
      status: 404,
    })
  })
})
