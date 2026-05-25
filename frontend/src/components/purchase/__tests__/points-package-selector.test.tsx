/**
 * @vitest-environment jsdom
 */

import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PointsPackageSelector } from '../points-package-selector'
import type { ExtPointsPackageItem } from '@/lib/api-generated'

function makeStandardExtPkg(overrides: Partial<ExtPointsPackageItem> = {}): ExtPointsPackageItem {
  return {
    id: 'pkg-1',
    name: 'basic',
    title: 'Basic Pack',
    description: null,
    points: 1000,
    price: 999,
    currency: 'USD',
    packageType: 'standard',
    originalPrice: null,
    discountPercent: null,
    promoStartTime: null,
    promoEndTime: null,
    sortOrder: 5,
    ...overrides,
  }
}

function makePromoExtPkg(overrides: Partial<ExtPointsPackageItem> = {}): ExtPointsPackageItem {
  return makeStandardExtPkg({
    id: 'pkg-2',
    name: 'spring-sale',
    title: 'Spring Sale',
    points: 2000,
    price: 999,
    currency: 'USD',
    packageType: 'promotional',
    originalPrice: 1999,
    discountPercent: 50,
    promoStartTime: '2026-05-01T00:00:00Z',
    promoEndTime: '2026-12-31T23:59:59Z',
    sortOrder: 10,
    ...overrides,
  })
}

const noop = () => {}

describe('PointsPackageSelector conditional promo rendering', () => {
  describe('discount badge', () => {
    it('does NOT render discount badge for standard packages', () => {
      render(
        <PointsPackageSelector
          packages={[makeStandardExtPkg()]}
          selectedPackageId={null}
          onSelect={noop}
        />
      )

      expect(screen.queryByTestId('points-package-discount-badge')).toBeNull()
    })

    it('renders discount badge ONLY for promotional packages with discountPercent', () => {
      render(
        <PointsPackageSelector
          packages={[makePromoExtPkg()]}
          selectedPackageId={null}
          onSelect={noop}
        />
      )

      expect(screen.getByTestId('points-package-discount-badge')).toBeInTheDocument()
    })

    it('does NOT render discount badge for promotional packages without discountPercent', () => {
      render(
        <PointsPackageSelector
          packages={[makePromoExtPkg({ discountPercent: null })]}
          selectedPackageId={null}
          onSelect={noop}
        />
      )

      expect(screen.queryByTestId('points-package-discount-badge')).toBeNull()
    })
  })

  describe('strikethrough original price', () => {
    it('does NOT render strikethrough price when originalPrice is null', () => {
      render(
        <PointsPackageSelector
          packages={[makeStandardExtPkg()]}
          selectedPackageId={null}
          onSelect={noop}
        />
      )

      // No element with "line-through" class when originalPrice is null
      const card = screen.getByTestId('points-package-card-pkg-1')
      const lineThroughElements = card.querySelectorAll('.line-through')
      expect(lineThroughElements).toHaveLength(0)
    })

    it('renders strikethrough price when originalPrice is present', () => {
      render(
        <PointsPackageSelector
          packages={[makePromoExtPkg()]}
          selectedPackageId={null}
          onSelect={noop}
        />
      )

      const card = screen.getByTestId('points-package-card-pkg-2')
      const lineThroughElements = card.querySelectorAll('.line-through')
      expect(lineThroughElements.length).toBeGreaterThan(0)
    })
  })

  describe('limited-time label', () => {
    it('does NOT render limited-time label for standard packages', () => {
      render(
        <PointsPackageSelector
          packages={[makeStandardExtPkg()]}
          selectedPackageId={null}
          onSelect={noop}
        />
      )

      expect(screen.queryByTestId('points-package-limited-time')).toBeNull()
    })

    it('renders limited-time label for packages with promoEndTime', () => {
      render(
        <PointsPackageSelector
          packages={[makePromoExtPkg()]}
          selectedPackageId={null}
          onSelect={noop}
        />
      )

      expect(screen.getByTestId('points-package-limited-time')).toBeInTheDocument()
    })
  })

  describe('mixed cards', () => {
    it('renders both standard and promotional cards without error', () => {
      const standard = makeStandardExtPkg()
      const promo = makePromoExtPkg()

      render(
        <PointsPackageSelector
          packages={[standard, promo]}
          selectedPackageId={null}
          onSelect={noop}
        />
      )

      expect(screen.getByTestId('points-package-card-pkg-1')).toBeInTheDocument()
      expect(screen.getByTestId('points-package-card-pkg-2')).toBeInTheDocument()
      expect(screen.getByTestId('points-package-discount-badge')).toBeInTheDocument()
      expect(screen.queryByTestId('points-package-limited-time')).toBeInTheDocument()
    })
  })
})
