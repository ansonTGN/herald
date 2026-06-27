import { describe, it, expect } from 'vitest'
import { selectPeriodPane, disabledReason } from '../purchase-points'
import { purchaseOptionsList } from '@/test/fixtures/entitlement-mappings'
import type { PurchaseOptionView } from '@/lib/api-generated'

// Unit-level contracts for the purchase-page pure helpers. The dev component
// test (purchase-points.test.tsx) drives these through the rendered period
// toggle and disabled card; here we assert the CONTRACT directly — the pinned
// one_time pane placement and the disabled-cause predicate. No MSW, no render.

describe('period pane selection', () => {
  // Pinned decision: recurring items appear ONLY in the pane whose
  // billingPeriod matches; one_time packs are period-agnostic and appear in
  // BOTH panes. Hiding a one_time pack under either toggle is a regression
  // vs. always listing it.

  it('selects the monthly recurring card and every one_time pack into the month pane', () => {
    const items = purchaseOptionsList()

    const monthPane = selectPeriodPane(items, 'month')

    const ids = monthPane.map((o) => o.mappingId)
    // Monthly recurring present.
    expect(ids).toContain('map_pro_monthly')
    // Annual recurring must NOT leak into the month pane.
    expect(ids).not.toContain('map_pro_annual')
    // one_time Creem pack is period-agnostic → present in BOTH panes.
    expect(ids).toContain('map_starter')
  })

  it('selects the annual recurring card and every one_time pack into the year pane', () => {
    const items = purchaseOptionsList()

    const yearPane = selectPeriodPane(items, 'year')

    const ids = yearPane.map((o) => o.mappingId)
    // Annual recurring present.
    expect(ids).toContain('map_pro_annual')
    // Monthly recurring must NOT leak into the year pane.
    expect(ids).not.toContain('map_pro_monthly')
    // The SAME one_time Creem pack also appears in the year pane.
    expect(ids).toContain('map_starter')
  })

  it('places a one_time pack in both panes (period-agnostic, pinned contract)', () => {
    const items: PurchaseOptionView[] = [
      {
        mappingId: 'ot',
        entitlementKey: 'pack',
        externalProductId: 'prod_pack',
        externalPriceId: null,
        paymentProvider: 'creem',
        billingType: 'one_time',
        billingPeriod: null,
        amount: 499,
        currency: 'usd',
        displayName: 'One-Time Pack',
        enabled: true,
        pointsPerPeriod: null,
      },
    ]

    const month = selectPeriodPane(items, 'month')
    const year = selectPeriodPane(items, 'year')

    // The same single one_time pack is visible under both toggles.
    expect(month.map((o) => o.mappingId)).toEqual(['ot'])
    expect(year.map((o) => o.mappingId)).toEqual(['ot'])
  })

  it('excludes a recurring card whose billingPeriod does not match', () => {
    const items: PurchaseOptionView[] = [
      {
        mappingId: 'annual_only',
        entitlementKey: 'pro',
        externalProductId: 'prod_pro',
        externalPriceId: 'pa',
        paymentProvider: 'stripe',
        billingType: 'recurring',
        billingPeriod: 'year',
        amount: 9999,
        currency: 'usd',
        displayName: 'Annual Only',
        enabled: true,
        pointsPerPeriod: null,
      },
    ]

    expect(selectPeriodPane(items, 'month')).toEqual([])
    expect(selectPeriodPane(items, 'year').map((o) => o.mappingId)).toEqual(['annual_only'])
  })
})

describe('disabled card reason', () => {
  // Predicate: reason returned when `!option.enabled || !option.paymentProvider`.
  // Note: PurchaseOptionView.paymentProvider is typed `string` (required) in the
  // generated DTO, so the "no provider" case is represented by an empty string
  // rather than `undefined` — both are falsy and exercise the same branch.
  it.each([
    {
      label: 'disabled mapping (enabled=false) with a provider set',
      option: { enabled: false, paymentProvider: 'stripe' },
      expectsReason: true,
    },
    {
      label: 'enabled mapping but no provider wired (empty string)',
      option: { enabled: true, paymentProvider: '' },
      expectsReason: true,
    },
    {
      label: 'enabled mapping with a provider',
      option: { enabled: true, paymentProvider: 'stripe' },
      expectsReason: false,
    },
  ])('returns $expectsReason reason flag for: $label', ({ option, expectsReason }) => {
    const base = purchaseOptionsList()[0]
    const merged: PurchaseOptionView = { ...base, ...option }

    const reason = disabledReason(merged)

    if (expectsReason) {
      expect(reason).toEqual({ key: 'purchase.not_enabled_reason' })
    } else {
      expect(reason).toBeNull()
    }
  })
})
