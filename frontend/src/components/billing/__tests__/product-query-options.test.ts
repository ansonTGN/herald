import { describe, it, expect } from 'vitest'
import {
  productsQueryOptions,
  productQueryOptions,
  productPlansQueryOptions,
} from '@/data/query-options'
import { QUERY_KEYS } from '@/lib/constants'

describe('productsQueryOptions', () => {
  it('GIVEN realmId only WHEN creating query options THEN should include realmId in query key', () => {
    const options = productsQueryOptions('realm-1')

    expect(options.queryKey).toEqual([QUERY_KEYS.BILLING_PRODUCTS, 'realm-1'])
  })

  it('GIVEN different realmIds WHEN creating query options THEN should produce different cache keys', () => {
    const options1 = productsQueryOptions('realm-1')
    const options2 = productsQueryOptions('realm-2')

    expect(options1.queryKey).not.toEqual(options2.queryKey)
  })
})

describe('productQueryOptions', () => {
  it('should include realmId and productId in query key', () => {
    const options = productQueryOptions('realm-1', 'product-1')

    expect(options.queryKey).toEqual([QUERY_KEYS.BILLING_PRODUCT, 'realm-1', 'product-1'])
  })

  it('GIVEN same productId but different realmIds WHEN creating query options THEN should produce different cache keys', () => {
    const options1 = productQueryOptions('realm-1', 'product-1')
    const options2 = productQueryOptions('realm-2', 'product-1')

    expect(options1.queryKey).not.toEqual(options2.queryKey)
  })
})
