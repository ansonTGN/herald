import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PlanFormPage } from '../plan-form-page'
import { ProductFormDialog } from '../product-form-dialog'
import type { PlanResponse } from '@/lib/api-generated'
import { server } from '@/test/mocks/server'

// ==================== Router Mock ====================

const mockNavigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => mockNavigate,
}))

// ==================== Test Helpers ====================

const REALM_ID = 'realm-1'
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

const mockProducts = [
  {
    id: 'prod-1',
    realmId: REALM_ID,
    name: 'starter',
    title: 'Starter',
    description: null,
    sortOrder: 1,
    enabled: true,
    plansCount: 1,
    createdAt: '2026-03-29T00:00:00Z',
    updatedAt: '2026-03-29T00:00:00Z',
  },
  {
    id: 'prod-2',
    realmId: REALM_ID,
    name: 'growth',
    title: 'Growth',
    description: null,
    sortOrder: 2,
    enabled: true,
    plansCount: 0,
    createdAt: '2026-03-29T00:00:00Z',
    updatedAt: '2026-03-29T00:00:00Z',
  },
]

function productsHandler() {
  return http.get(`${BASE_URL}/api/bill/${REALM_ID}/products`, () => {
    return HttpResponse.json({ products: mockProducts })
  })
}

const basePlan: PlanResponse = {
  id: 'plan-1',
  realmId: REALM_ID,
  productId: 'prod-1',
  name: 'starter-monthly',
  title: 'Starter Monthly',
  description: 'Starter monthly plan',
  type: 'monthly',
  price: 1000,
  currency: 'USD',
  sortOrder: 1,
  active: true,
  trialDays: 14,
  checkoutUrl: 'https://example.com/checkout/starter',
  createdAt: '2026-03-29T00:00:00Z',
  updatedAt: '2026-03-29T00:00:00Z',
}

const secondPlan: PlanResponse = {
  ...basePlan,
  id: 'plan-2',
  productId: 'prod-2',
  name: 'growth-yearly',
  title: 'Growth Yearly',
  description: 'Growth yearly plan',
  type: 'yearly',
  price: 12000,
  checkoutUrl: 'https://example.com/checkout/growth',
}

const firstProduct = {
  id: 'product-1',
  realmId: REALM_ID,
  code: 'starter',
  title: 'Starter Product',
  description: 'Starter description',
  enabled: true,
  plansCount: 1,
  createdAt: '2026-03-29T00:00:00Z',
  updatedAt: '2026-03-29T00:00:00Z',
}

const secondProduct = {
  ...firstProduct,
  id: 'product-2',
  code: 'growth',
  title: 'Growth Product',
  description: 'Growth description',
  sortOrder: 2,
  enabled: false,
}

describe('Billing form regressions', () => {
  beforeEach(() => {
    server.use(productsHandler())
  })

  // ==================== PlanFormPage regressions ====================

  describe('PlanFormPage', () => {
    it('allows changing product when editing an existing plan', async () => {
      let capturedBody: unknown = null
      server.use(
        productsHandler(),
        http.patch(`${BASE_URL}/api/bill/${REALM_ID}/plans/${basePlan.id}`, async ({ request }) => {
          capturedBody = await request.json()
          return HttpResponse.json(basePlan, { status: 200 })
        })
      )

      const user = userEvent.setup({ delay: null })
      renderWithProviders(<PlanFormPage mode="edit" realmId={REALM_ID} plan={basePlan} />)

      // Wait for the page to render with products loaded
      await waitFor(() => {
        expect(screen.getByTestId('plan-form-title')).toHaveTextContent('Edit Plan')
      })

      const trigger = screen.getByTestId('plan-product-select-trigger')
      expect(trigger).not.toBeDisabled()

      await user.click(trigger)
      await user.click(screen.getByTestId('plan-product-prod-2'))
      await user.click(screen.getByTestId('plan-form-submit-button'))

      await waitFor(() => {
        expect(capturedBody).not.toBeNull()
      })

      const body = capturedBody as Record<string, unknown>
      expect(body.productId).toBe('prod-2')
    })

    it('initializes form with correct values when mounted with a different plan', async () => {
      // First mount with basePlan
      const queryClient = createTestQueryClient()
      const { unmount } = renderWithProviders(
        <PlanFormPage mode="edit" realmId={REALM_ID} plan={basePlan} />,
        queryClient
      )

      await waitFor(() => {
        expect(screen.getByTestId('plan-form-title')).toHaveTextContent('Edit Plan')
        expect(screen.getByTestId('plan-name-input')).toHaveValue('starter-monthly')
        expect(screen.getByTestId('plan-title-input')).toHaveValue('Starter Monthly')
      })

      unmount()

      // Mount a fresh component with secondPlan (simulates navigating to a different edit page)
      renderWithProviders(
        <PlanFormPage mode="edit" realmId={REALM_ID} plan={secondPlan} />,
        queryClient
      )

      await waitFor(() => {
        expect(screen.getByTestId('plan-form-title')).toHaveTextContent('Edit Plan')
        expect(screen.getByTestId('plan-name-input')).toHaveValue('growth-yearly')
        expect(screen.getByTestId('plan-title-input')).toHaveValue('Growth Yearly')
      })
    })

    it('navigates to billing page on successful submit', async () => {
      server.use(
        productsHandler(),
        http.post(`${BASE_URL}/api/bill/${REALM_ID}/plans`, async () => {
          return HttpResponse.json(basePlan, { status: 201 })
        })
      )

      const user = userEvent.setup({ delay: null })
      renderWithProviders(<PlanFormPage mode="create" realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('plan-form-title')).toHaveTextContent('Create Plan')
      })

      // Fill required fields
      await user.click(screen.getByTestId('plan-product-select-trigger'))
      await user.click(screen.getByTestId('plan-product-prod-1'))
      await user.type(screen.getByTestId('plan-name-input'), 'test-plan')
      await user.type(screen.getByTestId('plan-title-input'), 'Test Plan')
      await user.type(screen.getByTestId('plan-price-input'), '1000')
      await user.click(screen.getByTestId('plan-form-submit-button'))

      await waitFor(() => {
        expect(mockNavigate).toHaveBeenCalledWith({
          to: '/$realmId/manage/billing',
          params: { realmId: REALM_ID },
          search: { status: 'all' },
        })
      })
    })

    it('navigates to billing page on cancel', async () => {
      const user = userEvent.setup({ delay: null })
      renderWithProviders(<PlanFormPage mode="create" realmId={REALM_ID} />)

      await waitFor(() => {
        expect(screen.getByTestId('plan-form-title')).toHaveTextContent('Create Plan')
      })

      await user.click(screen.getByTestId('plan-form-cancel-button'))

      expect(mockNavigate).toHaveBeenCalledWith({
        to: '/$realmId/manage/billing',
        params: { realmId: REALM_ID },
        search: { status: 'all' },
      })
    })
  })

  // ==================== ProductFormDialog regressions (unchanged) ====================

  describe('ProductFormDialog', () => {
    it('resets product form values when switching between edit targets and create mode', async () => {
      const user = userEvent.setup({ delay: null })
      const { rerender } = render(
        <ProductFormDialog
          product={firstProduct}
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={vi.fn()}
          isSubmitting={false}
        />
      )

      const titleInput = screen.getByTestId('product-title-input')
      await user.clear(titleInput)
      await user.type(titleInput, 'Unsaved Product Title')
      expect(titleInput).toHaveValue('Unsaved Product Title')

      rerender(
        <ProductFormDialog
          product={secondProduct}
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={vi.fn()}
          isSubmitting={false}
        />
      )

      await waitFor(() => {
        expect(screen.getByTestId('product-code-input')).toHaveValue('growth')
        expect(screen.getByTestId('product-title-input')).toHaveValue('Growth Product')
      })

      rerender(
        <ProductFormDialog
          product={undefined}
          open={true}
          onOpenChange={vi.fn()}
          onSubmit={vi.fn()}
          isSubmitting={false}
        />
      )

      await waitFor(() => {
        expect(screen.getByTestId('product-code-input')).toHaveValue('')
        expect(screen.getByTestId('product-title-input')).toHaveValue('')
      })
    })
  })
})
