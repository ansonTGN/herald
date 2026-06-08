import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { PurchaseHistoryList } from '../purchase-history-list'
import type { PurchaseHistoryItemDto } from '@/lib/api-generated'

const purchase: PurchaseHistoryItemDto = {
  attemptId: 'attempt-1',
  targetMappingId: 'mapping-1',
  productName: 'Test Product',
  points: 100,
  amount: 999,
  currency: 'USD',
  paymentProvider: 'stripe',
  status: 'Succeeded',
  completedAt: '2025-01-01T00:05:00Z',
  createdAt: '2025-01-01T00:00:00Z',
}

describe('PurchaseHistoryList invoice action', () => {
  it('renders invoice button when onApplyInvoice is provided', () => {
    render(
      <PurchaseHistoryList
        purchases={[purchase]}
        isLoading={false}
        onDetailsClick={vi.fn()}
        onApplyInvoice={vi.fn()}
      />
    )

    expect(screen.getByTestId('purchase-history-invoice-button-attempt-1')).toBeInTheDocument()
  })

  it('calls onApplyInvoice with purchase.attemptId', async () => {
    const user = userEvent.setup()
    const onApplyInvoice = vi.fn()

    render(
      <PurchaseHistoryList
        purchases={[purchase]}
        isLoading={false}
        onDetailsClick={vi.fn()}
        onApplyInvoice={onApplyInvoice}
      />
    )

    await user.click(screen.getByTestId('purchase-history-invoice-button-attempt-1'))

    expect(onApplyInvoice).toHaveBeenCalledTimes(1)
    expect(onApplyInvoice).toHaveBeenCalledWith('attempt-1')
  })
})

describe('PurchaseHistoryList status badge', () => {
  it('renders status badge with correct variant for Succeeded', () => {
    render(
      <PurchaseHistoryList purchases={[purchase]} isLoading={false} onDetailsClick={vi.fn()} />
    )

    const badge = screen.getByText('Succeeded')
    expect(badge).toBeInTheDocument()
  })

  it('renders status badge for Failed', () => {
    const failedPurchase = { ...purchase, attemptId: 'attempt-2', status: 'Failed' }
    render(
      <PurchaseHistoryList
        purchases={[failedPurchase]}
        isLoading={false}
        onDetailsClick={vi.fn()}
      />
    )

    expect(screen.getByText('Failed')).toBeInTheDocument()
  })
})

describe('PurchaseHistoryList null handling', () => {
  it('renders -- for null points', () => {
    const noPointsPurchase = { ...purchase, attemptId: 'attempt-3', points: null }
    render(
      <PurchaseHistoryList
        purchases={[noPointsPurchase]}
        isLoading={false}
        onDetailsClick={vi.fn()}
      />
    )

    expect(screen.getByText('--')).toBeInTheDocument()
  })

  it('renders i18n fallback text for null productName', () => {
    const noNamePurchase = { ...purchase, attemptId: 'attempt-4', productName: null }
    render(
      <PurchaseHistoryList
        purchases={[noNamePurchase]}
        isLoading={false}
        onDetailsClick={vi.fn()}
      />
    )

    // Should show localized fallback text instead of UUID fragment
    expect(screen.getByText('Unknown Product')).toBeInTheDocument()
  })
})
