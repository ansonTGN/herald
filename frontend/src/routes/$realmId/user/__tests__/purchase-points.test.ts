import { describe, it, expect } from 'vitest'
import { resolveMappingForProvider, providersForEntitlement } from '../purchase-points'
import type { OneTimeMappingItem, PaymentProviderInfo } from '@/lib/api-generated'

/**
 * Intent encoded here (Rule 9): an entitlement with one mapping per provider
 * MUST resolve to the mapping whose provider matches the user's payment-method
 * choice — never the originally-clicked card's provider. Backend rejects a
 * targetMappingId whose provider disagrees with the requested paymentProvider
 * (design §4.2.2), so submitting a cross-provider pair is always wrong.
 *
 * Mirrors the demo seed (scripts/lib/demo_seed.py): the `credits-500`
 * entitlement has two mappings, one for `stripe` and one for `creem`.
 */
function makeMapping(entitlementKey: string, provider: string, id: string): OneTimeMappingItem {
  return {
    id,
    entitlementKey,
    paymentProvider: provider,
    bucketId: 'bucket-1',
    providerProductInfo: { name: `${entitlementKey} via ${provider}`, price: 500, currency: 'USD' },
  }
}

const mappings: OneTimeMappingItem[] = [
  makeMapping('credits-500', 'creem', 'map-credits-500-creem'),
  makeMapping('credits-500', 'stripe', 'map-credits-500-stripe'),
  makeMapping('credits-1000', 'stripe', 'map-credits-1000-stripe'),
]

const allProviders: PaymentProviderInfo[] = [
  { platform: 'stripe' },
  { platform: 'creem' },
  { platform: 'paypal' },
]

describe('resolveMappingForProvider', () => {
  it('returns the stripe mapping when the user picks stripe for credits-500', () => {
    expect(resolveMappingForProvider(mappings, 'credits-500', 'stripe')).toBe(
      'map-credits-500-stripe'
    )
  })

  it('returns the creem mapping when the user picks creem for credits-500', () => {
    expect(resolveMappingForProvider(mappings, 'credits-500', 'creem')).toBe(
      'map-credits-500-creem'
    )
  })

  it('returns undefined when the entitlement has no mapping for the provider (unbuyable combo)', () => {
    // credits-1000 is stripe-only; picking creem must NOT fall back to the
    // stripe mapping (that is exactly the bug being fixed).
    expect(resolveMappingForProvider(mappings, 'credits-1000', 'creem')).toBeUndefined()
  })

  it('does not fall back to a different entitlement when the provider mismatches', () => {
    // Regression guard: the old code submitted the clicked card's mappingId
    // regardless of the picked provider. Here the entitlement is unknown but a
    // mapping for the provider exists under another entitlement — must NOT
    // cross-resolve.
    expect(resolveMappingForProvider(mappings, 'credits-9999', 'stripe')).toBeUndefined()
  })

  it('returns undefined when inputs are missing', () => {
    expect(resolveMappingForProvider(undefined, 'credits-500', 'stripe')).toBeUndefined()
    expect(resolveMappingForProvider(mappings, null, 'stripe')).toBeUndefined()
    expect(resolveMappingForProvider(mappings, 'credits-500', null)).toBeUndefined()
  })
})

describe('providersForEntitlement', () => {
  it('lists only providers that have a mapping for the selected entitlement', () => {
    const result = providersForEntitlement(mappings, 'credits-500', allProviders)
    expect(result.map((p) => p.platform).sort()).toEqual(['creem', 'stripe'])
  })

  it('excludes configured-but-unmapped providers (latent UX bug guard)', () => {
    // paypal is configured realm-wide but credits-500 is not offered via it.
    const result = providersForEntitlement(mappings, 'credits-500', allProviders)
    expect(result.find((p) => p.platform === 'paypal')).toBeUndefined()
  })

  it('returns only stripe for a stripe-only entitlement', () => {
    const result = providersForEntitlement(mappings, 'credits-1000', allProviders)
    expect(result.map((p) => p.platform)).toEqual(['stripe'])
  })

  it('returns empty when no entitlement is selected', () => {
    expect(providersForEntitlement(mappings, null, allProviders)).toEqual([])
  })
})
