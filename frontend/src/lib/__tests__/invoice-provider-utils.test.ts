import { describe, it, expect } from 'vitest'
import {
  isExternalInvoice,
  getProviderLabel,
  getViewInProviderUrl,
  getAvailableActions,
} from '../invoice-utils'

// ---------- Factory helpers ----------

function makeInvoice(
  overrides: Partial<{
    provider: string
    externalHostedUrl: string | null | undefined
  }>
): { provider: string; externalHostedUrl?: string | null } {
  return {
    provider: overrides.provider ?? 'manual',
    externalHostedUrl: overrides.externalHostedUrl,
  }
}

// ==================== isExternalInvoice ====================

describe('isExternalInvoice', () => {
  it('returns false for manual — the boundary between external and internal', () => {
    expect(isExternalInvoice('manual')).toBe(false)
  })

  it.each(['stripe', 'creem', 'wechat'] as const)(
    'returns true for known external provider %s',
    (provider) => {
      expect(isExternalInvoice(provider)).toBe(true)
    }
  )

  it('returns true for unknown provider — any non-manual is external', () => {
    expect(isExternalInvoice('future_provider')).toBe(true)
  })
})

// ==================== getProviderLabel ====================

describe('getProviderLabel', () => {
  it('returns a mapped label for manual — not the raw string', () => {
    const label = getProviderLabel('manual')
    expect(label).toBeTruthy()
    expect(label).not.toBe('manual')
  })

  it.each([
    { provider: 'stripe' as const, expected: 'Stripe' },
    { provider: 'creem' as const, expected: 'Creem' },
  ])('returns brand name $expected for provider $provider', ({ provider, expected }) => {
    expect(getProviderLabel(provider)).toBe(expected)
  })

  it('returns a mapped label for wechat — not the raw string', () => {
    const label = getProviderLabel('wechat')
    expect(label).toBeTruthy()
    expect(label).not.toBe('wechat')
  })

  it('returns the raw provider string for unknown providers — fallback passthrough', () => {
    expect(getProviderLabel('future_provider')).toBe('future_provider')
  })
})

// ==================== getViewInProviderUrl ====================

describe('getViewInProviderUrl', () => {
  it('returns null for manual provider invoice even when externalHostedUrl is set', () => {
    const invoice = makeInvoice({ provider: 'manual', externalHostedUrl: 'https://example.com' })
    expect(getViewInProviderUrl(invoice)).toBeNull()
  })

  it('returns the externalHostedUrl for external provider with a URL present', () => {
    const invoice = makeInvoice({
      provider: 'stripe',
      externalHostedUrl: 'https://stripe.com/invoice/123',
    })
    expect(getViewInProviderUrl(invoice)).toBe('https://stripe.com/invoice/123')
  })

  it('returns null for external provider when externalHostedUrl is null', () => {
    const invoice = makeInvoice({ provider: 'stripe', externalHostedUrl: null })
    expect(getViewInProviderUrl(invoice)).toBeNull()
  })

  it('returns null for external provider when externalHostedUrl is undefined', () => {
    const invoice = makeInvoice({ provider: 'stripe' })
    expect(getViewInProviderUrl(invoice)).toBeNull()
  })
})

// ==================== getAvailableActions ====================

describe('getAvailableActions', () => {
  describe('backward compatibility — manual or undefined provider', () => {
    it.each([
      { status: 'draft', provider: undefined as string | undefined },
      { status: 'draft', provider: 'manual' },
    ] as const)(
      'returns status-based actions for status=$status provider=$provider',
      ({ status, provider }) => {
        expect(getAvailableActions(status, provider)).toEqual(['view', 'edit', 'issue', 'void'])
      }
    )

    it('returns full actions for issued status without external provider', () => {
      expect(getAvailableActions('issued', undefined)).toEqual([
        'view',
        'void',
        'markPaid',
        'downloadPdf',
      ])
      expect(getAvailableActions('issued', 'manual')).toEqual([
        'view',
        'void',
        'markPaid',
        'downloadPdf',
      ])
    })

    it('returns view + downloadPdf for paid status without external provider', () => {
      expect(getAvailableActions('paid', undefined)).toEqual(['view', 'downloadPdf'])
    })

    it('returns view + void + markPaid + downloadPdf for overdue status without external provider', () => {
      expect(getAvailableActions('overdue', undefined)).toEqual([
        'view',
        'void',
        'markPaid',
        'downloadPdf',
      ])
    })

    it('returns only view for void status', () => {
      expect(getAvailableActions('void', undefined)).toEqual(['view'])
    })
  })

  describe('external provider restriction', () => {
    it.each(['stripe', 'creem'] as const)(
      'returns only [view] for external provider %s regardless of status',
      (provider) => {
        expect(getAvailableActions('draft', provider)).toEqual(['view'])
        expect(getAvailableActions('issued', provider)).toEqual(['view'])
        expect(getAvailableActions('paid', provider)).toEqual(['view'])
        expect(getAvailableActions('overdue', provider)).toEqual(['view'])
        expect(getAvailableActions('void', provider)).toEqual(['view'])
      }
    )
  })

  it('returns status-based actions for empty string provider — falsy falls through', () => {
    expect(getAvailableActions('draft', '')).toEqual(['view', 'edit', 'issue', 'void'])
  })

  it('returns [view] as fallback for unknown status without provider', () => {
    expect(getAvailableActions('unknown_status', undefined)).toEqual(['view'])
  })

  it('returns [view] as fallback for unknown status with external provider', () => {
    expect(getAvailableActions('unknown_status', 'stripe')).toEqual(['view'])
  })
})
