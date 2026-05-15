import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { HistoryEventBadge } from '../history-event-badge'

describe('HistoryEventBadge - Label Display', () => {
  it('should display correct label for each event type', () => {
    const labels: Record<string, string> = {
      created: 'Created',
      upgraded: 'Upgraded',
      downgraded: 'Downgraded',
      canceled: 'Canceled',
      expired: 'Expired',
      renewed: 'Renewed',
      reactivated: 'Reactivated',
      billing_period_changed: 'Billing Period Changed',
    }

    Object.entries(labels).forEach(([type, label]) => {
      const { unmount } = render(<HistoryEventBadge eventType={type as any} />)
      expect(screen.getByText(label)).toBeInTheDocument()
      unmount()
    })
  })
})
