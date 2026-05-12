import { fireEvent, render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SubscriptionHistoryFilter } from '../subscription-history-filter'

describe('SubscriptionHistoryFilter', () => {
  it('renders with empty optional selects without crashing', () => {
    render(
      <SubscriptionHistoryFilter
        filters={{}}
        onFiltersChange={vi.fn()}
        onReset={vi.fn()}
        plans={[{ id: 'plan-1', name: 'Starter' }]}
      />
    )

    expect(screen.getByTestId('subscription-history-filter')).toBeInTheDocument()
    expect(screen.getByLabelText('Plan')).toBeInTheDocument()
    expect(screen.getByLabelText('Event Type')).toBeInTheDocument()
    expect(screen.getByLabelText('Subscription Status')).toBeInTheDocument()
  })

  it('converts date-only input values to UTC ISO timestamps on submit', async () => {
    const onFiltersChange = vi.fn()

    render(
      <SubscriptionHistoryFilter filters={{}} onFiltersChange={onFiltersChange} onReset={vi.fn()} />
    )

    fireEvent.change(screen.getByTestId('filter-from-date'), { target: { value: '2025-01-01' } })
    fireEvent.change(screen.getByTestId('filter-to-date'), { target: { value: '2025-01-31' } })
    await userEvent.click(screen.getByTestId('apply-filter-button'))

    expect(onFiltersChange).toHaveBeenCalledWith(
      expect.objectContaining({
        fromDate: '2025-01-01T00:00:00.000Z',
        toDate: '2025-01-31T23:59:59.999Z',
      })
    )
  })
})
