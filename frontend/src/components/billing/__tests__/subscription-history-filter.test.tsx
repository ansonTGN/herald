import { fireEvent, render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SubscriptionHistoryFilter } from '../subscription-history-filter'

describe('SubscriptionHistoryFilter', () => {
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
