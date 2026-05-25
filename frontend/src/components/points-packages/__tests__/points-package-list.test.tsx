/**
 * @vitest-environment jsdom
 */

import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { PointsPackageList } from '../points-package-list'
import type { PointsPackageResponse } from '@/lib/api-generated'

function makeStandardPkg(overrides: Partial<PointsPackageResponse> = {}): PointsPackageResponse {
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
    enabled: true,
    realmId: 'realm-1',
    sortOrder: 0,
    isExpired: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

function makePromoPkg(overrides: Partial<PointsPackageResponse> = {}): PointsPackageResponse {
  return makeStandardPkg({
    id: 'pkg-2',
    name: 'spring-sale',
    title: 'Spring Sale',
    packageType: 'promotional',
    originalPrice: 2000,
    discountPercent: 50,
    promoStartTime: '2026-05-01T00:00:00Z',
    promoEndTime: '2026-06-30T23:59:59Z',
    isExpired: false,
    sortOrder: 10,
    ...overrides,
  })
}

const noop = () => {}

describe('PointsPackageList smoke test with mixed data', () => {
  it('renders without error with mixed standard and promotional packages', () => {
    const packages = [makeStandardPkg(), makePromoPkg()]

    render(
      <PointsPackageList
        data={packages}
        isLoading={false}
        onEdit={noop}
        onDelete={noop}
        onConfigureProviders={noop}
      />
    )

    // Table renders with both packages
    expect(screen.getByTestId('points-packages-table')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-edit-button-pkg-1')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-edit-button-pkg-2')).toBeInTheDocument()
  })
})
