/**
 * API Key Query Options Tests
 *
 * Tests query key structure, cache key isolation, and parameter mapping
 * for apiKeysQueryOptions and apiKeyQueryOptions.
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { queryKeys, apiKeysQueryOptions, apiKeyQueryOptions } from '../query-options'

// Mock the API functions to observe parameters in queryFn tests
vi.mock('@/lib/api-generated', async () => {
  const actual = await vi.importActual<typeof import('@/lib/api-generated')>('@/lib/api-generated')
  return {
    ...actual,
    listApiKeys: vi.fn(),
    getApiKey: vi.fn(),
  }
})

import { listApiKeys, getApiKey } from '@/lib/api-generated'

// Minimal factory for successful listApiKeys response
function makeListResponse(overrides?: Record<string, unknown>) {
  return {
    data: {
      items: [],
      total: 0,
      page: 0,
      pageSize: 20,
      ...overrides,
    },
    error: undefined,
  }
}

// Minimal factory for successful getApiKey response
function makeDetailResponse(overrides?: Record<string, unknown>) {
  return {
    data: {
      id: 'key-1',
      name: 'Test Key',
      enabled: true,
      realmId: 'realm-1',
      createdAt: '2025-01-01T00:00:00Z',
      ...overrides,
    },
    error: undefined,
  }
}

describe('API Key Query Options', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('queryKeys.apiKeys -- cache key isolation', () => {
    it('produces different keys for different realm IDs', () => {
      const keyA = queryKeys.apiKeys('realm-a', {})
      const keyB = queryKeys.apiKeys('realm-b', {})
      expect(keyA).not.toEqual(keyB)
    })

    it('produces different keys for different filter objects', () => {
      const keyEmpty = queryKeys.apiKeys('realm-1', {})
      const keyPaged = queryKeys.apiKeys('realm-1', { page: 2 })
      const keySized = queryKeys.apiKeys('realm-1', { pageSize: 50 })
      expect(keyEmpty).not.toEqual(keyPaged)
      expect(keyEmpty).not.toEqual(keySized)
      expect(keyPaged).not.toEqual(keySized)
    })

    it('produces different keys for different page numbers', () => {
      const keyPage0 = queryKeys.apiKeys('realm-1', { page: 0 })
      const keyPage3 = queryKeys.apiKeys('realm-1', { page: 3 })
      expect(keyPage0).not.toEqual(keyPage3)
    })

    it('produces different keys for different page sizes', () => {
      const keySize20 = queryKeys.apiKeys('realm-1', { pageSize: 20 })
      const keySize50 = queryKeys.apiKeys('realm-1', { pageSize: 50 })
      expect(keySize20).not.toEqual(keySize50)
    })

    it('produces identical keys for identical parameters (deterministic)', () => {
      const filters = { page: 1, pageSize: 30 }
      const keyA = queryKeys.apiKeys('realm-1', filters)
      const keyB = queryKeys.apiKeys('realm-1', filters)
      expect(keyA).toEqual(keyB)
    })
  })

  describe('queryKeys.apiKey -- detail key', () => {
    it('produces different keys for different API Key IDs', () => {
      const keyA = queryKeys.apiKey('realm-1', 'key-a')
      const keyB = queryKeys.apiKey('realm-1', 'key-b')
      expect(keyA).not.toEqual(keyB)
    })

    it('produces different keys for different realm IDs', () => {
      const keyA = queryKeys.apiKey('realm-a', 'key-1')
      const keyB = queryKeys.apiKey('realm-b', 'key-1')
      expect(keyA).not.toEqual(keyB)
    })
  })

  describe('apiKeysQueryOptions -- parameter mapping', () => {
    it('sends default pagination (page 0, pageSize 20) when no filters provided', async () => {
      vi.mocked(listApiKeys).mockResolvedValueOnce(makeListResponse() as any)

      const options = apiKeysQueryOptions('realm-1', {})
      await options.queryFn!()

      expect(listApiKeys).toHaveBeenCalledWith({
        path: { realmId: 'realm-1' },
        query: { page: 0, pageSize: 20 },
      })
    })

    it('passes explicit filter parameters to the API call', async () => {
      vi.mocked(listApiKeys).mockResolvedValueOnce(
        makeListResponse({ page: 2, pageSize: 50 }) as any
      )

      const options = apiKeysQueryOptions('realm-1', { page: 2, pageSize: 50 })
      await options.queryFn!()

      expect(listApiKeys).toHaveBeenCalledWith({
        path: { realmId: 'realm-1' },
        query: { page: 2, pageSize: 50 },
      })
    })

    it('defaults page to 0 when only pageSize is provided', async () => {
      vi.mocked(listApiKeys).mockResolvedValueOnce(makeListResponse() as any)

      const options = apiKeysQueryOptions('realm-1', { pageSize: 10 })
      await options.queryFn!()

      expect(listApiKeys).toHaveBeenCalledWith({
        path: { realmId: 'realm-1' },
        query: { page: 0, pageSize: 10 },
      })
    })

    it('defaults pageSize to 20 when only page is provided', async () => {
      vi.mocked(listApiKeys).mockResolvedValueOnce(makeListResponse() as any)

      const options = apiKeysQueryOptions('realm-1', { page: 3 })
      await options.queryFn!()

      expect(listApiKeys).toHaveBeenCalledWith({
        path: { realmId: 'realm-1' },
        query: { page: 3, pageSize: 20 },
      })
    })
  })

  describe('apiKeyQueryOptions -- parameter mapping', () => {
    it('calls getApiKey with correct realmId and apiKeyId', async () => {
      vi.mocked(getApiKey).mockResolvedValueOnce(makeDetailResponse() as any)

      const options = apiKeyQueryOptions('realm-1', 'key-42')
      await options.queryFn!()

      expect(getApiKey).toHaveBeenCalledWith({
        path: { realmId: 'realm-1', apiKeyId: 'key-42' },
      })
    })
  })
})
