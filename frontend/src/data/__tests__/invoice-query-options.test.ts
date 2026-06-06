import { describe, it, expect } from 'vitest'
import {
  invoiceListQueryOptions,
  myInvoiceListQueryOptions,
  invoiceKeys,
} from '../invoice-query-options'

const REALM_ID = 'realm-123'

// ==================== invoiceListQueryOptions (admin) ====================

describe('invoiceListQueryOptions', () => {
  it('query key without provider matches existing shape — backward compat', () => {
    const options = invoiceListQueryOptions(REALM_ID)
    const key = options.queryKey

    expect(key[0]).toBe('invoices')
    expect(key[1]).toBe(REALM_ID)
    expect(key[2]).toBe('list')
    // query parameter is undefined when no filters are provided
    expect(key[3]).toBeUndefined()
  })

  it('query key with provider: "stripe" includes provider in the query object', () => {
    const options = invoiceListQueryOptions(REALM_ID, { provider: 'stripe' })
    const key = options.queryKey

    expect(key[3]).toEqual(expect.objectContaining({ provider: 'stripe' }))
  })

  it('query key with provider differs from query key without provider — cache isolation', () => {
    const withProvider = invoiceListQueryOptions(REALM_ID, { provider: 'stripe' })
    const withoutProvider = invoiceListQueryOptions(REALM_ID)

    expect(withProvider.queryKey).not.toEqual(withoutProvider.queryKey)
  })

  it('query key with provider "stripe" differs from query key with provider "manual" — per-provider isolation', () => {
    const stripe = invoiceListQueryOptions(REALM_ID, { provider: 'stripe' })
    const manual = invoiceListQueryOptions(REALM_ID, { provider: 'manual' })

    expect(stripe.queryKey).not.toEqual(manual.queryKey)
  })

  it('invoiceKeys.list includes provider when passed in query', () => {
    const key = invoiceKeys.list(REALM_ID, { provider: 'stripe' })
    expect(key).toEqual(['invoices', REALM_ID, 'list', { provider: 'stripe' }])
  })

  it('query key with status and provider includes both filters', () => {
    const options = invoiceListQueryOptions(REALM_ID, { status: 'issued', provider: 'stripe' })
    const key = options.queryKey

    expect(key[3]).toEqual(expect.objectContaining({ status: 'issued', provider: 'stripe' }))
  })
})

// ==================== myInvoiceListQueryOptions (user) ====================

describe('myInvoiceListQueryOptions', () => {
  it('query key without provider matches existing shape — backward compat', () => {
    const options = myInvoiceListQueryOptions(REALM_ID)
    const key = options.queryKey

    expect(key[0]).toBe('invoices')
    expect(key[1]).toBe(REALM_ID)
    expect(key[2]).toBe('my')
    expect(key[3]).toBe('list')
    // query parameter is undefined when no filters are provided
    expect(key[4]).toBeUndefined()
  })

  it('query key with provider: "stripe" includes provider in the query object', () => {
    const options = myInvoiceListQueryOptions(REALM_ID, { provider: 'stripe' })
    const key = options.queryKey

    expect(key[4]).toEqual(expect.objectContaining({ provider: 'stripe' }))
  })

  it('query key with provider differs from query key without provider — cache isolation', () => {
    const withProvider = myInvoiceListQueryOptions(REALM_ID, { provider: 'stripe' })
    const withoutProvider = myInvoiceListQueryOptions(REALM_ID)

    expect(withProvider.queryKey).not.toEqual(withoutProvider.queryKey)
  })

  it('query key with provider "stripe" differs from query key with provider "manual" — per-provider isolation', () => {
    const stripe = myInvoiceListQueryOptions(REALM_ID, { provider: 'stripe' })
    const manual = myInvoiceListQueryOptions(REALM_ID, { provider: 'manual' })

    expect(stripe.queryKey).not.toEqual(manual.queryKey)
  })
})
