import { describe, it, expect } from 'vitest'
import { disabledReason } from '../purchase-points'
import { purchaseOptionsList } from '@/test/fixtures/entitlement-mappings'
import type { PurchaseOptionView } from '@/lib/api-generated'

// Unit-level contract for the purchase-page disabled-reason predicate. The dev
// component test (purchase-points.test.tsx) drives this through a rendered
// disabled card; here we assert the CONTRACT directly. No MSW, no render.

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
