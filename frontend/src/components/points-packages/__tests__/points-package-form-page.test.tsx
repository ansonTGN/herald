/**
 * @vitest-environment jsdom
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PointsPackageFormPage } from '../points-package-form-page'
import type { PointsPackageResponse } from '@/lib/api-generated'
import { server } from '@/test/mocks/server'

// ==================== Router Mock ====================

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// ==================== Test Helpers ====================

const REALM_ID = 'test-realm'
const BASE_URL = 'http://localhost:3000'

function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  })
}

function renderWithProviders(ui: React.ReactElement, queryClient?: QueryClient) {
  const qc = queryClient ?? createTestQueryClient()
  return render(<QueryClientProvider client={qc}>{ui}</QueryClientProvider>)
}

function makePointsPackage(overrides: Partial<PointsPackageResponse> = {}): PointsPackageResponse {
  return {
    id: 'pkg-1',
    realmId: REALM_ID,
    name: 'basic-package',
    title: 'Basic Points Package',
    description: 'A great starter package',
    points: 100,
    price: 999,
    currency: 'USD',
    sortOrder: 0,
    enabled: true,
    packageType: 'standard',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
    ...overrides,
  }
}

function createPackageHandler() {
  return http.post(`${BASE_URL}/api/bill/${REALM_ID}/points-packages`, async () => {
    return HttpResponse.json({ data: makePointsPackage() }, { status: 201 })
  })
}

function updatePackageHandler(packageId: string = 'pkg-1') {
  return http.patch(`${BASE_URL}/api/bill/${REALM_ID}/points-packages/${packageId}`, async () => {
    return HttpResponse.json({ data: makePointsPackage() })
  })
}

// ==================== Tests ====================

describe('PointsPackageFormPage promo validation', () => {
  beforeEach(() => {
    server.use(createPackageHandler(), updatePackageHandler())
  })

  describe('promotional package validation', () => {
    it('shows error when promotional package is submitted without originalPrice', async () => {
      const user = userEvent.setup()
      renderWithProviders(<PointsPackageFormPage mode="create" realmId={REALM_ID} />)

      // Wait for form to render
      await waitFor(() => {
        expect(screen.getByText('Create Points Package')).toBeInTheDocument()
      })

      // Fill required base fields
      await user.type(screen.getByTestId('points-package-name-input'), 'promo-pkg')
      await user.type(screen.getByTestId('points-package-title-input'), 'Promo Package')
      await user.clear(screen.getByTestId('points-package-points-input'))
      await user.type(screen.getByTestId('points-package-points-input'), '100')
      await user.clear(screen.getByTestId('points-package-price-input'))
      await user.type(screen.getByTestId('points-package-price-input'), '9.99')
      await user.type(screen.getByTestId('points-package-currency-select'), 'USD')

      // Switch to promotional type
      await user.click(screen.getByTestId('points-package-type-promotional'))

      // Wait for promo fields to appear
      await waitFor(() => {
        expect(screen.getByTestId('points-package-original-price-input')).toBeInTheDocument()
      })

      // Fill promoEndTime but leave originalPrice empty
      await user.type(screen.getByTestId('points-package-promo-end-input'), '2026-12-31T23:59')

      // Submit
      await user.click(screen.getByTestId('points-package-submit-button'))

      // Should show validation error for originalPrice
      await waitFor(() => {
        const errorElements = screen.getAllByText(/original price is required/i)
        expect(errorElements.length).toBeGreaterThan(0)
      })
    })

    it('shows error when originalPrice is less than or equal to selling price', async () => {
      const user = userEvent.setup()
      renderWithProviders(<PointsPackageFormPage mode="create" realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByText('Create Points Package')).toBeInTheDocument()
      })

      // Fill required base fields
      await user.type(screen.getByTestId('points-package-name-input'), 'promo-pkg')
      await user.type(screen.getByTestId('points-package-title-input'), 'Promo Package')
      await user.clear(screen.getByTestId('points-package-points-input'))
      await user.type(screen.getByTestId('points-package-points-input'), '100')
      await user.clear(screen.getByTestId('points-package-price-input'))
      await user.type(screen.getByTestId('points-package-price-input'), '9.99')
      await user.type(screen.getByTestId('points-package-currency-select'), 'USD')

      // Switch to promotional type
      await user.click(screen.getByTestId('points-package-type-promotional'))

      await waitFor(() => {
        expect(screen.getByTestId('points-package-original-price-input')).toBeInTheDocument()
      })

      // Set originalPrice to same as price (9.99) -- must be strictly greater
      await user.type(screen.getByTestId('points-package-original-price-input'), '9.99')
      await user.type(screen.getByTestId('points-package-promo-end-input'), '2026-12-31T23:59')

      // Submit
      await user.click(screen.getByTestId('points-package-submit-button'))

      await waitFor(() => {
        const errorElements = screen.getAllByText(
          /original price must be greater than the selling price/i
        )
        expect(errorElements.length).toBeGreaterThan(0)
      })
    })

    it('shows error when promotional package is submitted without promoEndTime', async () => {
      const user = userEvent.setup()
      renderWithProviders(<PointsPackageFormPage mode="create" realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByText('Create Points Package')).toBeInTheDocument()
      })

      // Fill required base fields
      await user.type(screen.getByTestId('points-package-name-input'), 'promo-pkg')
      await user.type(screen.getByTestId('points-package-title-input'), 'Promo Package')
      await user.clear(screen.getByTestId('points-package-points-input'))
      await user.type(screen.getByTestId('points-package-points-input'), '100')
      await user.clear(screen.getByTestId('points-package-price-input'))
      await user.type(screen.getByTestId('points-package-price-input'), '9.99')
      await user.type(screen.getByTestId('points-package-currency-select'), 'USD')

      // Switch to promotional type
      await user.click(screen.getByTestId('points-package-type-promotional'))

      await waitFor(() => {
        expect(screen.getByTestId('points-package-original-price-input')).toBeInTheDocument()
      })

      // Fill originalPrice but leave promoEndTime empty
      await user.type(screen.getByTestId('points-package-original-price-input'), '19.99')

      // Submit
      await user.click(screen.getByTestId('points-package-submit-button'))

      await waitFor(() => {
        const errorElements = screen.getAllByText(/promo end time is required/i)
        expect(errorElements.length).toBeGreaterThan(0)
      })
    })
  })

  describe('edit mode promo defaults', () => {
    it('loads existing promo package with promo fields pre-filled', async () => {
      renderWithProviders(
        <PointsPackageFormPage
          mode="edit"
          realmId={REALM_ID}
          pkg={makePointsPackage({
            packageType: 'promotional',
            originalPrice: 1999,
            promoStartTime: '2026-06-01T00:00',
            promoEndTime: '2026-12-31T23:59',
          })}
        />
      )

      await waitFor(() => {
        expect(screen.getByText('Edit Points Package')).toBeInTheDocument()
      })

      // Verify promotional radio is selected (both radio items exist)
      const promoRadio = screen.getByTestId('points-package-type-promotional') as HTMLInputElement
      expect(promoRadio).toBeChecked()

      // Verify promo fields are visible and populated
      // originalPrice: API stores 1999 cents => display 19.99
      const originalPriceInput = screen.getByTestId(
        'points-package-original-price-input'
      ) as HTMLInputElement
      expect(originalPriceInput.value).toBe('19.99')

      const promoStartInput = screen.getByTestId(
        'points-package-promo-start-input'
      ) as HTMLInputElement
      expect(promoStartInput.value).toBe('2026-06-01T00:00')

      const promoEndInput = screen.getByTestId('points-package-promo-end-input') as HTMLInputElement
      expect(promoEndInput.value).toBe('2026-12-31T23:59')
    })
  })
})
