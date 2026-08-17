import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import type { PurchaseOptionView } from '@/lib/api-generated'
import { CurrencyPurchaseGroup } from '../currency-purchase-group'
import { groupByEntitlement } from '../currency-utils'
import { makePurchaseOption as makeOption } from '@/test/fixtures/purchase-option'

// Behavior contracts for the per-entitlement currency purchase block:
//   - The switcher only exists for all-Stripe multi-currency entitlements;
//     store-priced providers (Creem/IAP/WeChat) and single-currency products
//     degrade to the plain price list.
//   - There is no default currency: with multiple currencies no price rows
//     show until the user explicitly picks one, so purchase can only ever
//     target a deliberately chosen currency's mapping id. A single currency
//     is auto-selected because it is the only choice.

function renderGroup(options: PurchaseOptionView[]) {
  const [group] = groupByEntitlement(options)
  if (!group) throw new Error('fixture produced no group')
  const onSelect = vi.fn()
  const view = render(
    <CurrencyPurchaseGroup group={group} selectedMappingId={null} onSelect={onSelect} />
  )
  return { onSelect, ...view }
}

describe('currency switcher rendering', () => {
  it('renders a switcher for an all-Stripe multi-currency entitlement', () => {
    renderGroup([
      makeOption({ mappingId: 'm-usd', externalPriceId: 'price_usd', currency: 'usd' }),
      makeOption({ mappingId: 'm-eur', externalPriceId: 'price_eur', currency: 'eur' }),
    ])

    expect(screen.getByTestId('purchase-currency-switch-pro-plan')).toBeInTheDocument()
    expect(screen.getByTestId('purchase-currency-option-pro-plan-usd')).toBeInTheDocument()
    expect(screen.getByTestId('purchase-currency-option-pro-plan-eur')).toBeInTheDocument()
  })

  it('renders no switcher for a store-priced (Creem) entitlement and keeps its single price visible', () => {
    renderGroup([
      makeOption({
        mappingId: 'm-creem',
        entitlementKey: 'pack',
        externalPriceId: null,
        paymentProvider: 'creem',
        currency: null,
        amount: null,
      }),
    ])

    expect(screen.queryByTestId('purchase-currency-switch-pack')).toBeNull()
    // The degraded block still renders the price row for purchase.
    expect(screen.getByTestId('purchase-price-card-m-creem')).toBeInTheDocument()
  })

  it('renders no switcher for a single-currency Stripe product', () => {
    renderGroup([
      makeOption({ externalPriceId: 'price_m', currency: 'usd', billingPeriod: 'month' }),
      makeOption({ externalPriceId: 'price_y', currency: 'usd', billingPeriod: 'year' }),
    ])

    expect(screen.queryByTestId('purchase-currency-switch-pro-plan')).toBeNull()
    // Same-currency billing periods coexist in the group (user still picks
    // the period inside the currency group).
    expect(screen.getByTestId('purchase-price-card-price_m')).toBeInTheDocument()
    expect(screen.getByTestId('purchase-price-card-price_y')).toBeInTheDocument()
  })
})

describe('explicit currency selection', () => {
  const multiCurrency = [
    makeOption({
      mappingId: 'm-usd',
      externalPriceId: 'price_usd',
      currency: 'usd',
    }),
    makeOption({ mappingId: 'm-eur', externalPriceId: 'price_eur', currency: 'eur' }),
  ]

  it('shows no price rows until a currency is explicitly picked', () => {
    renderGroup(multiCurrency)

    expect(screen.getByTestId('purchase-currency-select-prompt-pro-plan')).toBeInTheDocument()
    expect(screen.queryByTestId('purchase-price-card-price_usd')).toBeNull()
    expect(screen.queryByTestId('purchase-price-card-price_eur')).toBeNull()
  })

  it('shows the picked currency cards and unmounts the other currency cards', async () => {
    const user = userEvent.setup()
    renderGroup(multiCurrency)

    await user.click(screen.getByTestId('purchase-currency-option-pro-plan-usd'))

    expect(screen.getByTestId('purchase-price-card-price_usd')).toBeInTheDocument()
    expect(screen.queryByTestId('purchase-price-card-price_eur')).toBeNull()
  })
})

describe('card selection', () => {
  it('propagates the clicked row mapping id as the purchase target', async () => {
    const user = userEvent.setup()
    const { onSelect } = renderGroup([
      makeOption({ mappingId: 'm-usd', externalPriceId: 'price_usd', currency: 'usd' }),
      makeOption({ mappingId: 'm-eur', externalPriceId: 'price_eur', currency: 'eur' }),
    ])

    await user.click(screen.getByTestId('purchase-currency-option-pro-plan-usd'))
    await user.click(screen.getByTestId('purchase-price-card-price_usd'))

    expect(onSelect).toHaveBeenCalledWith('m-usd')
  })
})
