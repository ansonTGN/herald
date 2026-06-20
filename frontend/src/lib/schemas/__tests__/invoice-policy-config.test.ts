import { describe, it, expect } from 'vitest'
import { invoicePolicyConfigSchema, getInvoicePolicyDefaults } from '../invoice-forms'

// ==================== invoicePolicyConfigSchema ====================

describe('invoicePolicyConfigSchema', () => {
  it('accepts valid { policy: "provider_first" } — minimal input, providerCapabilities defaults to {}', () => {
    const result = invoicePolicyConfigSchema.safeParse({ policy: 'provider_first' })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.policy).toBe('provider_first')
      expect(result.data.providerCapabilities).toEqual({})
    }
  })

  it('accepts valid { policy: "manual_only" }', () => {
    const result = invoicePolicyConfigSchema.safeParse({ policy: 'manual_only' })
    expect(result.success).toBe(true)
  })

  it('accepts valid { policy: "none" }', () => {
    const result = invoicePolicyConfigSchema.safeParse({ policy: 'none' })
    expect(result.success).toBe(true)
  })

  it('rejects invalid policy value — enum enforcement', () => {
    const result = invoicePolicyConfigSchema.safeParse({ policy: 'invalid_policy' })
    expect(result.success).toBe(false)
  })

  it('rejects missing policy field — required field', () => {
    const result = invoicePolicyConfigSchema.safeParse({})
    expect(result.success).toBe(false)
  })

  it('accepts providerCapabilities with valid structure', () => {
    const result = invoicePolicyConfigSchema.safeParse({
      policy: 'provider_first',
      providerCapabilities: {
        stripe: { externalInvoiceEnabled: true },
      },
    })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.providerCapabilities.stripe.externalInvoiceEnabled).toBe(true)
    }
  })

  it('accepts empty providerCapabilities: {}', () => {
    const result = invoicePolicyConfigSchema.safeParse({
      policy: 'provider_first',
      providerCapabilities: {},
    })
    expect(result.success).toBe(true)
  })

  it('defaults providerCapabilities to {} when omitted', () => {
    const result = invoicePolicyConfigSchema.safeParse({ policy: 'manual_only' })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(result.data.providerCapabilities).toEqual({})
    }
  })

  it('accepts providerCapabilities with multiple providers', () => {
    const result = invoicePolicyConfigSchema.safeParse({
      policy: 'provider_first',
      providerCapabilities: {
        stripe: { externalInvoiceEnabled: true },
        creem: { externalInvoiceEnabled: true },
      },
    })
    expect(result.success).toBe(true)
    if (result.success) {
      expect(Object.keys(result.data.providerCapabilities)).toHaveLength(2)
    }
  })

  it('rejects providerCapabilities with non-boolean externalInvoiceEnabled', () => {
    const result = invoicePolicyConfigSchema.safeParse({
      policy: 'provider_first',
      providerCapabilities: {
        stripe: { externalInvoiceEnabled: 'yes' },
      },
    })
    expect(result.success).toBe(false)
  })
})

// ==================== getInvoicePolicyDefaults ====================

describe('getInvoicePolicyDefaults', () => {
  it('returns policy: "provider_first" as default policy', () => {
    const defaults = getInvoicePolicyDefaults()
    expect(defaults.policy).toBe('provider_first')
  })

  it('returns providerCapabilities containing all known providers', () => {
    const defaults = getInvoicePolicyDefaults()
    expect(defaults.providerCapabilities.stripe.externalInvoiceEnabled).toBe(true)
    expect(defaults.providerCapabilities.creem.externalInvoiceEnabled).toBe(true)
    expect(defaults.providerCapabilities.shopify).toBeUndefined()
  })

  it('returned object passes schema validation', () => {
    const defaults = getInvoicePolicyDefaults()
    const result = invoicePolicyConfigSchema.safeParse(defaults)
    expect(result.success).toBe(true)
  })
})
