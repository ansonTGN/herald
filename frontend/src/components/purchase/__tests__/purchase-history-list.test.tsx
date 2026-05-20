import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { PurchaseHistoryList } from '../purchase-history-list'
import type { PurchaseHistoryItemDto } from '@/lib/api-generated'

const purchase: PurchaseHistoryItemDto = {
  id: 'purchase-1',
  realmId: 'realm-1',
  userId: 'user-1',
  pointsPackageId: 'package-1',
  paymentAttemptId: '11111111-1111-1111-1111-111111111111',
  points: 100,
  amount: 10,
  currency: 'CNY',
  paymentProvider: 'stripe',
  pointsTransactionId: null,
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

    expect(screen.getByTestId('purchase-history-invoice-button-purchase-1')).toBeInTheDocument()
  })

  it('calls onApplyInvoice with purchase.paymentAttemptId', async () => {
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

    await user.click(screen.getByTestId('purchase-history-invoice-button-purchase-1'))

    expect(onApplyInvoice).toHaveBeenCalledTimes(1)
    expect(onApplyInvoice).toHaveBeenCalledWith('11111111-1111-1111-1111-111111111111')
  })
})
