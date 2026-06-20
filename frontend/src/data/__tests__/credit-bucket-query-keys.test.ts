import { describe, it, expect } from 'vitest'
import { queryKeys, pointsTransactionsQueryOptions } from '@/data/query-options'

// ==================== Factory functions ====================

function makeRealmId(suffix: string = '1'): string {
  return `realm-${suffix}`
}

function makeBucketId(suffix: string = '1'): string {
  return `bucket-${suffix}`
}

// ==================== credit-bucket query keys ====================

describe('credit-bucket query keys', () => {
  describe('list vs detail isolation (same realmId)', () => {
    it('distinguishes list key from detail key', () => {
      const realmId = makeRealmId()
      const listKey = queryKeys.creditBucketsList(realmId)
      const detailKey = queryKeys.creditBucket(realmId, makeBucketId())

      // list vs detail must not collide: detail has an extra bucketId segment,
      // so they must not deep-equal (otherwise list/detail share a cache entry).
      expect(listKey).not.toEqual(detailKey)
    })

    it('is stable for identical list input', () => {
      const realmId = makeRealmId()
      expect(queryKeys.creditBucketsList(realmId)).toEqual(queryKeys.creditBucketsList(realmId))
    })

    it('is stable for identical detail input', () => {
      const realmId = makeRealmId()
      const bucketId = makeBucketId()
      expect(queryKeys.creditBucket(realmId, bucketId)).toEqual(
        queryKeys.creditBucket(realmId, bucketId)
      )
    })
  })

  describe('overview vs list/detail isolation', () => {
    it('distinguishes overview key from list key', () => {
      const realmId = makeRealmId()
      expect(queryKeys.creditBucketOverview(realmId)).not.toEqual(
        queryKeys.creditBucketsList(realmId)
      )
    })

    it('distinguishes overview key from detail key', () => {
      const realmId = makeRealmId()
      expect(queryKeys.creditBucketOverview(realmId)).not.toEqual(
        queryKeys.creditBucket(realmId, makeBucketId())
      )
    })
  })

  describe('cross-realm isolation', () => {
    it('distinguishes list keys across different realms', () => {
      // Realm scoping must enter the cache key: same list shape, different realmId
      // must yield different keys (otherwise one realm's buckets leak into another).
      expect(queryKeys.creditBucketsList(makeRealmId('1'))).not.toEqual(
        queryKeys.creditBucketsList(makeRealmId('2'))
      )
    })
  })
})

// ==================== pointsTransactions bucketId cache isolation ====================

describe('pointsTransactionsQueryOptions query key', () => {
  it('isolates cache when filters include bucketId vs not', () => {
    const realmId = makeRealmId()

    // bucketId now drives history filtering (FE-D01); it must enter the query key
    // so per-bucket history does not collide with realm-wide history in the cache.
    const withoutBucket = pointsTransactionsQueryOptions(realmId, {}).queryKey
    const withBucket = pointsTransactionsQueryOptions(realmId, { bucketId: makeBucketId() }).queryKey

    expect(withoutBucket).not.toEqual(withBucket)
  })
})
