/**
 * @vitest-environment jsdom
 */

import { describe, expect, it } from 'vitest'
import type { QuotaWindowViewDto, WalletByBucketResponse } from '@/lib/api-generated'
import { deriveUserPointsView } from '@/components/points/user-points-view'

/**
 * Factory for a single wallet row, mirroring the one in
 * `cross-bucket-derivation.test.ts` but extended so the quota-window fields
 * introduced in design §4.2.2 can be injected per-test.
 */
function makeWalletByBucket(
  overrides: Partial<WalletByBucketResponse> & { userId: string }
): WalletByBucketResponse {
  return {
    bucketId: 'bucket-a',
    name: 'Default',
    enabled: true,
    bucketTotal: 0,
    balancesByType: {
      freePeriodic: 0,
      granted: 0,
      registration: 0,
      subscription: 0,
      topup: 0,
    },
    // Default to pool-only semantics: no quota windows, no spendable-from-quota,
    // pool-side balance unspecified. Individual tests override these.
    quotaWindows: null,
    spendableFromQuota: null,
    spendableFromPool: null,
    ...overrides,
  }
}

/** Factory for a single backend-precomputed quota window (design §4.2.2). */
function makeQuotaWindow(
  overrides: Partial<QuotaWindowViewDto> & { key: string }
): QuotaWindowViewDto {
  return {
    limit: 100,
    used: 0,
    remaining: 100,
    windowSeconds: 30 * 24 * 60 * 60,
    isTightest: false,
    exhausted: false,
    resetsAt: null,
    ...overrides,
  }
}

const CURRENT_USER = 'user-self'
const OTHER_USER = 'user-other'

describe('deriveUserPointsView — quota-window pass-through (§4.2.2)', () => {
  describe('pool-only buckets: zero-regression for absent quota fields', () => {
    it.each([
      ['null', null],
      ['omitted', undefined],
    ])(
      'passes quotaWindows=%s and spendableFromQuota=%s through untouched on a pool-only bucket',
      (_label, quotaValue) => {
        // INTENT: a pool-only bucket carries no quota entitlement. The
        // dashboard must NOT interpret an absent quota as zero spendable, nor
        // fabricate an empty window array — that would break the pool-only
        // total. `deriveUserPointsView` is a pass-through, so the absent-quota
        // signal must survive derivation verbatim.
        const items: WalletByBucketResponse[] = [
          makeWalletByBucket({
            userId: CURRENT_USER,
            bucketId: 'pool-only',
            bucketTotal: 50,
            quotaWindows: quotaValue as WalletByBucketResponse['quotaWindows'],
            spendableFromQuota: null,
            spendableFromPool: 50,
          }),
        ]

        const [card] = deriveUserPointsView(items, CURRENT_USER).cards

        expect(card.quotaWindows).toBe(quotaValue)
        expect(card.spendableFromQuota).toBeNull()
        expect(card.spendableFromPool).toBe(50)
        // bucketTotal is the backend-computed total — NOT recomputed here.
        expect(card.bucketTotal).toBe(50)
      }
    )

    it('preserves an explicitly empty quotaWindows array as empty (does not coerce to null)', () => {
      // INTENT: if the backend ever signals Some([]) (no active windows but a
      // quota-bearing bucket), the pass-through must not silently rewrite it
      // to null — that would change downstream rendering semantics. We assert
      // the value is carried as-is.
      const items: WalletByBucketResponse[] = [
        makeWalletByBucket({
          userId: CURRENT_USER,
          bucketId: 'quota-empty',
          bucketTotal: 0,
          quotaWindows: [],
          spendableFromQuota: 0,
          spendableFromPool: 0,
        }),
      ]

      const [card] = deriveUserPointsView(items, CURRENT_USER).cards

      expect(card.quotaWindows).toEqual([])
      expect(card.spendableFromQuota).toBe(0)
      expect(card.spendableFromPool).toBe(0)
    })
  })

  describe('single window: backend-precomputed fields pass through verbatim', () => {
    it('carries the window limit/used/remaining/resetsAt/isTightest/exhausted untouched', () => {
      // INTENT: the dashboard (FE-D03) renders `remaining`, `isTightest`,
      // `resetsAt` etc. directly from the backend `QuotaWindowViewDto`.
      // `deriveUserPointsView` MUST NOT recompute the tightest constraint or
      // reset time client-side — doing so would desync from the authoritative
      // backend computation. This test pins the pass-through so any future
      // client-side recomputation surfaces as a regression.
      const window: QuotaWindowViewDto = makeQuotaWindow({
        key: 'monthly',
        limit: 100,
        used: 30,
        remaining: 70,
        windowSeconds: 30 * 24 * 60 * 60,
        isTightest: true,
        exhausted: false,
        resetsAt: '2026-07-29T00:00:00Z',
      })

      const items: WalletByBucketResponse[] = [
        makeWalletByBucket({
          userId: CURRENT_USER,
          bucketId: 'single-window',
          bucketTotal: 120, // 70 quota + 50 pool, backend-computed
          quotaWindows: [window],
          spendableFromQuota: 70,
          spendableFromPool: 50,
        }),
      ]

      const [card] = deriveUserPointsView(items, CURRENT_USER).cards

      expect(card.quotaWindows).toHaveLength(1)
      expect(card.quotaWindows?.[0]).toEqual(window)
      expect(card.spendableFromQuota).toBe(70)
      expect(card.spendableFromPool).toBe(50)
      // bucketTotal stays as the backend total; the function does NOT verify
      // quota+pool == bucketTotal.
      expect(card.bucketTotal).toBe(120)
    })
  })

  describe('multiple windows: NO client-side min/tightest recomputation', () => {
    it('passes every window through unchanged even when the backend tightest flag disagrees with a naive client min', () => {
      // INTENT: a tempting bug would be for the client to recompute
      // `isTightest` = argmin(remaining) and overwrite the backend flag. The
      // contract is that the backend OWNS the tightest decision (it may factor
      // in things beyond raw remaining). Here the second window has the
      // smaller remaining but the backend flagged the first as tightest — the
      // pass-through must preserve the backend's authoritative flag exactly.
      const windows: QuotaWindowViewDto[] = [
        makeQuotaWindow({
          key: 'daily',
          remaining: 40,
          isTightest: true, // backend authority
        }),
        makeQuotaWindow({
          key: 'monthly',
          remaining: 10, // smaller, but NOT flagged tightest by backend
          isTightest: false,
        }),
      ]

      const items: WalletByBucketResponse[] = [
        makeWalletByBucket({
          userId: CURRENT_USER,
          bucketId: 'multi-window',
          bucketTotal: 60,
          quotaWindows: windows,
          spendableFromQuota: 40, // backend's chosen min, not 10
          spendableFromPool: 20,
        }),
      ]

      const [card] = deriveUserPointsView(items, CURRENT_USER).cards

      expect(card.quotaWindows).toEqual(windows)
      // spendableFromQuota is passed through, NOT recomputed as min(remaining)=10.
      expect(card.spendableFromQuota).toBe(40)
    })
  })

  describe('quota contribution folds into cross-bucket aggregation unchanged', () => {
    it('sums backend bucketTotal across current-user buckets including the quota-folded total', () => {
      // INTENT: the only client-side aggregation is crossBucketTotal (sum of
      // backend bucketTotal). Since bucketTotal already folds in the window
      // contribution server-side, the cross-bucket sum must reflect the
      // extended semantics with NO special-casing here. Other users' rows
      // (with their own bucketTotals) must not leak.
      const items: WalletByBucketResponse[] = [
        makeWalletByBucket({
          userId: CURRENT_USER,
          bucketId: 'quota-bucket',
          bucketTotal: 120, // 70 quota + 50 pool, backend-computed
          quotaWindows: [makeQuotaWindow({ key: 'm', remaining: 70, isTightest: true })],
          spendableFromQuota: 70,
          spendableFromPool: 50,
        }),
        makeWalletByBucket({
          userId: CURRENT_USER,
          bucketId: 'pool-bucket',
          bucketTotal: 30,
          quotaWindows: null,
          spendableFromQuota: null,
          spendableFromPool: 30,
        }),
        makeWalletByBucket({
          userId: OTHER_USER,
          bucketId: 'other-bucket',
          bucketTotal: 9999, // must NOT leak
        }),
      ]

      const result = deriveUserPointsView(items, CURRENT_USER)

      expect(result.crossBucketTotal).toBe(150)
      expect(result.showTotalBar).toBe(true)
    })
  })
})
