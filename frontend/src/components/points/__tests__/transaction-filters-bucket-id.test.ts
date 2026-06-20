/**
 * bucketId URL search-param parsing (TransactionFilters / design §4.4.2, §4.2.3).
 *
 * SUT = `transactionBucketSearchSchema` exported from `@/lib/schemas/points-forms`
 * (FE-D06 contract), which backs the `/$realmId/user/points` route's
 * `validateSearch`. Tests the business semantic that a malformed URL `bucketId`
 * is not let through — not zod library guarantees.
 */
import { describe, it, expect } from 'vitest'
import { transactionBucketSearchSchema } from '@/lib/schemas/points-forms'

/** Fixed valid UUID v4 used across assertions (no runtime RNG flakiness). */
const VALID_BUCKET_ID = '550e8400-e29b-41d4-a716-446655440000'

describe('transactionBucketSearchSchema — bucketId parsing', () => {
  describe('accepts shareable bucket dimension', () => {
    it.each([
      { input: {}, label: 'no bucketId (all-buckets state)', expected: undefined },
      { input: { bucketId: VALID_BUCKET_ID }, label: 'valid UUID', expected: VALID_BUCKET_ID },
    ])('parses OK for $label', ({ input, expected }) => {
      const result = transactionBucketSearchSchema.safeParse(input)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.bucketId).toBe(expected)
      }
    })
  })

  describe('rejects malformed URL bucketId', () => {
    // A bad `?bucketId=` in the URL must not silently become a filter value;
    // parse failure lets the route fall back to default (all buckets).
    it.each([
      { input: { bucketId: '' }, label: 'empty string' },
      { input: { bucketId: 'not-a-uuid' }, label: 'non-UUID' },
      { input: { bucketId: 'abc' }, label: 'short garbage' },
    ])('fails for $label', ({ input }) => {
      const result = transactionBucketSearchSchema.safeParse(input)
      expect(result.success).toBe(false)
    })
  })
})
