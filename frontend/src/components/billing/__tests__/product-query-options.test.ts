import { describe, it, expect } from 'vitest'
import {
  productsQueryOptions,
  productQueryOptions,
  productPlansQueryOptions,
} from '@/data/query-options'
import { QUERY_KEYS } from '@/lib/constants'

describe('productsQueryOptions', () => {
  describe('Query Configuration', () => {
    it('GIVEN realmId only WHEN creating query options THEN should include realmId in query key', () => {
      const options = productsQueryOptions('realm-1')

      expect(options.queryKey).toEqual([QUERY_KEYS.BILLING_PRODUCTS, 'realm-1'])
    })

    it('should configure retry count', () => {
      const options = productsQueryOptions('realm-1')

      expect(options.retry).toBe(1)
    })

    it('should configure stale time', () => {
      const options = productsQueryOptions('realm-1')

      expect(options.staleTime).toBe(1000 * 60 * 5) // 5 minutes
    })

    it('should have a query function', () => {
      const options = productsQueryOptions('realm-1')

      expect(options.queryFn).toBeDefined()
      expect(typeof options.queryFn).toBe('function')
    })
  })

  describe('Cache Key Isolation', () => {
    it('GIVEN different realmIds WHEN creating query options THEN should produce different cache keys', () => {
      const options1 = productsQueryOptions('realm-1')
      const options2 = productsQueryOptions('realm-2')

      expect(options1.queryKey).not.toEqual(options2.queryKey)
    })
  })

  describe('Query Key Structure', () => {
    it('should follow consistent naming pattern with billing-products prefix', () => {
      const options = productsQueryOptions('test-realm')

      expect(options.queryKey[0]).toBe(QUERY_KEYS.BILLING_PRODUCTS)
      expect(options.queryKey[1]).toBe('test-realm')
    })
  })
})

describe('productQueryOptions', () => {
  it('should include realmId and productId in query key', () => {
    const options = productQueryOptions('realm-1', 'product-1')

    expect(options.queryKey).toEqual([QUERY_KEYS.BILLING_PRODUCT, 'realm-1', 'product-1'])
  })

  it('GIVEN different productIds WHEN creating query options THEN should produce different cache keys', () => {
    const options1 = productQueryOptions('realm-1', 'product-1')
    const options2 = productQueryOptions('realm-1', 'product-2')

    expect(options1.queryKey).not.toEqual(options2.queryKey)
  })

  it('GIVEN same productId but different realmIds WHEN creating query options THEN should produce different cache keys', () => {
    const options1 = productQueryOptions('realm-1', 'product-1')
    const options2 = productQueryOptions('realm-2', 'product-1')

    expect(options1.queryKey).not.toEqual(options2.queryKey)
  })
})

describe('productPlansQueryOptions', () => {
  it('should include realmId and productId in query key', () => {
    const options = productPlansQueryOptions('realm-1', 'product-1')

    expect(options.queryKey).toEqual([QUERY_KEYS.BILLING_PRODUCT_PLANS, 'realm-1', 'product-1'])
  })

  it('GIVEN different productIds WHEN creating query options THEN should produce different cache keys', () => {
    const options1 = productPlansQueryOptions('realm-1', 'product-1')
    const options2 = productPlansQueryOptions('realm-1', 'product-2')

    expect(options1.queryKey).not.toEqual(options2.queryKey)
  })
})
