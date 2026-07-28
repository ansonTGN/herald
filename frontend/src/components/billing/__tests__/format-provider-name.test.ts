import { describe, it, expect } from 'vitest'
import { formatProviderName } from '../format-provider-name'

describe('formatProviderName', () => {
  it("returns 'App Store' for 'apple' (support-iap)", () => {
    expect(formatProviderName('apple')).toBe('App Store')
  })

  it("returns 'Google Play' for 'google' (support-iap)", () => {
    expect(formatProviderName('google')).toBe('Google Play')
  })

  // Regression: the IAP additions must not have disturbed the existing
  // explicit provider-name map.
  it('preserves the existing stripe/creem explicit mappings (regression)', () => {
    expect(formatProviderName('stripe')).toBe('Stripe')
    expect(formatProviderName('creem')).toBe('Creem')
  })

  it('falls back to capitalize() for an unknown provider (regression)', () => {
    expect(formatProviderName('acme')).toBe('Acme')
  })
})
