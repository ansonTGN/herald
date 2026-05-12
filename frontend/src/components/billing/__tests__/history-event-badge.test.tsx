import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { HistoryEventBadge } from '../history-event-badge'

describe('HistoryEventBadge - Color Mapping', () => {
  const eventTypeColors = [
    { type: 'created', expectedClass: 'bg-green-100' },
    { type: 'upgraded', expectedClass: 'bg-blue-100' },
    { type: 'downgraded', expectedClass: 'bg-orange-100' },
    { type: 'canceled', expectedClass: 'bg-red-100' },
    { type: 'renewed', expectedClass: 'bg-green-100' },
    { type: 'reactivated', expectedClass: 'bg-purple-100' },
    { type: 'expired', expectedClass: 'bg-gray-100' },
    { type: 'billing_period_changed', expectedClass: 'bg-cyan-100' },
  ]

  eventTypeColors.forEach(({ type, expectedClass }) => {
    it(`should apply correct color class for event type: ${type}`, () => {
      render(<HistoryEventBadge eventType={type as any} />)
      const badge = screen.getByTestId(`event-badge-${type}`)
      expect(badge).toHaveClass(expectedClass)
    })
  })
})

describe('HistoryEventBadge - Size Variants', () => {
  it('should apply sm size classes', () => {
    render(<HistoryEventBadge eventType="created" size="sm" />)
    const badge = screen.getByTestId('event-badge-created')
    expect(badge).toHaveClass('text-xs', 'px-2', 'py-0.5')
  })

  it('should apply md size classes (default)', () => {
    render(<HistoryEventBadge eventType="created" size="md" />)
    const badge = screen.getByTestId('event-badge-created')
    expect(badge).toHaveClass('text-xs', 'px-2.5', 'py-0.5')
  })

  it('should apply lg size classes', () => {
    render(<HistoryEventBadge eventType="created" size="lg" />)
    const badge = screen.getByTestId('event-badge-created')
    expect(badge).toHaveClass('text-sm', 'px-3', 'py-1')
  })
})

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

describe('HistoryEventBadge - Border Styles', () => {
  const eventTypeBorders = [
    { type: 'created', expectedClass: 'border-green-200' },
    { type: 'upgraded', expectedClass: 'border-blue-200' },
    { type: 'downgraded', expectedClass: 'border-orange-200' },
    { type: 'canceled', expectedClass: 'border-red-200' },
  ]

  eventTypeBorders.forEach(({ type, expectedClass }) => {
    it(`should apply correct border class for event type: ${type}`, () => {
      render(<HistoryEventBadge eventType={type as any} />)
      const badge = screen.getByTestId(`event-badge-${type}`)
      expect(badge).toHaveClass(expectedClass)
    })
  })
})

describe('HistoryEventBadge - Common Classes', () => {
  it('should apply common badge styling', () => {
    render(<HistoryEventBadge eventType="created" />)
    const badge = screen.getByTestId('event-badge-created')

    expect(badge).toHaveClass('inline-flex', 'items-center', 'rounded-md', 'border', 'font-medium')
  })
})
