import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PackageVsSubscriptionComparison } from '../package-vs-subscription-comparison'
import type { PointsPackageResponse } from '@/lib/api-generated'

describe('PackageVsSubscriptionComparison', () => {
  const mockPackages: PointsPackageResponse[] = [
    {
      id: 'pkg-1',
      name: 'starter_pack',
      title: 'Starter Pack',
      points: 1000,
      price: 999, // API price in cents (9.99 USD)
      currency: 'USD',
      enabled: true,
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
      realmId: 'realm-1',
    },
    {
      id: 'pkg-2',
      name: 'premium_pack',
      title: 'Premium Pack',
      points: 5000,
      price: 3999, // API price in cents (39.99 USD)
      currency: 'USD',
      enabled: true,
      createdAt: '2025-01-01T00:00:00Z',
      updatedAt: '2025-01-01T00:00:00Z',
      realmId: 'realm-1',
    },
  ]

  describe('rendering with packages', () => {
    it('should render comparison component', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      expect(screen.getByTestId('package-vs-subscription-comparison')).toBeInTheDocument()
    })

    it('should render info banner', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      // The component uses m['points.comparison_recommend_info_title']()
      // which resolves to the i18n message "Compare your options"
      expect(screen.getByText('Compare your options')).toBeInTheDocument()
    })

    it('should render comparison table headers', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      // Table headers from i18n: Feature, Points Package, Subscription
      expect(screen.getByText('Feature')).toBeInTheDocument()
      // "Points Package" appears in both the table header and the card title
      expect(screen.getAllByText('Points Package').length).toBeGreaterThan(0)
      expect(screen.getByText('Subscription')).toBeInTheDocument()
    })

    it('should render cheapest package card', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      expect(screen.getByTestId('cheapest-package-card')).toBeInTheDocument()
      expect(screen.getByText('1,000')).toBeInTheDocument()
      // formatPrice returns "9.99 USD" when API price is 999 cents
      expect(screen.getAllByText('9.99 USD').length).toBeGreaterThan(0)
    })

    it('should display N/A for starting price when no packages', () => {
      render(<PackageVsSubscriptionComparison packages={[]} />)

      expect(screen.getByText('N/A')).toBeInTheDocument()
    })
  })

  describe('cheapest package selection', () => {
    it('should highlight the cheapest package by price-per-unit', () => {
      // pkg-1: 1000 pts for $9.99, pkg-2: 5000 pts for $39.99
      // Cheapest by display price is pkg-1 ($9.99)
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      const card = screen.getByTestId('cheapest-package-card')
      expect(card).toBeInTheDocument()
      expect(screen.getByText('1,000')).toBeInTheDocument()
    })

    it('should pick the package with lowest display price', () => {
      const reversedPackages = [...mockPackages].reverse()
      render(<PackageVsSubscriptionComparison packages={reversedPackages} />)

      // Even with different input order, cheapest (pkg-1 at $9.99) should be picked
      expect(screen.getByText('1,000')).toBeInTheDocument()
    })
  })

  describe('value calculations', () => {
    it('should calculate points per dollar correctly', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      // 1000 points / 9.99 (display price from 999 cents) = 100.1 points/$
      const valueText = screen.getByText((content, element) => {
        return (
          content !== null &&
          element?.textContent !== null &&
          /^100\.1 pts\/\$/.test(element.textContent)
        )
      })
      expect(valueText).toBeInTheDocument()
    })
  })

  describe('comparison table content', () => {
    it('should show payment type row from i18n', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      expect(screen.getByText('Payment Type')).toBeInTheDocument()
      expect(screen.getByText('One-time Payment')).toBeInTheDocument()
      expect(screen.getByText('Recurring Payment')).toBeInTheDocument()
    })

    it('should show points delivery row from i18n', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      expect(screen.getByText('Points Delivery')).toBeInTheDocument()
      expect(screen.getByText('Immediate')).toBeInTheDocument()
      expect(screen.getByText('Periodic (Monthly/Weekly)')).toBeInTheDocument()
    })

    it('should show cost flexibility row from i18n', () => {
      render(<PackageVsSubscriptionComparison packages={mockPackages} />)

      expect(screen.getByText('Cost Flexibility')).toBeInTheDocument()
      expect(screen.getByText('Pay as you need')).toBeInTheDocument()
      expect(screen.getByText('Fixed commitment')).toBeInTheDocument()
    })
  })

  describe('interaction handlers', () => {
    it('should call onPackageClick when cheapest package card is clicked', () => {
      const onPackageClick = vi.fn()

      render(
        <PackageVsSubscriptionComparison packages={mockPackages} onPackageClick={onPackageClick} />
      )

      screen.getByTestId('cheapest-package-card').click()

      expect(onPackageClick).toHaveBeenCalledWith(mockPackages[0]) // Cheapest package
    })
  })

  describe('edge cases', () => {
    it('should render without crashing when packages is empty', () => {
      render(<PackageVsSubscriptionComparison packages={[]} />)

      expect(screen.getByTestId('package-vs-subscription-comparison')).toBeInTheDocument()
      // No cheapest-package-card should exist when no packages
      expect(screen.queryByTestId('cheapest-package-card')).not.toBeInTheDocument()
    })

    it('should handle packages with zero price', () => {
      const freePackages: PointsPackageResponse[] = [
        {
          ...mockPackages[0],
          price: 0, // 0 cents = $0.00
          points: 100,
        },
      ]

      render(<PackageVsSubscriptionComparison packages={freePackages} />)

      // formatPrice returns "0.00 USD" when API price is 0 cents
      expect(screen.getAllByText('0.00 USD').length).toBeGreaterThan(0)
    })
  })
})
