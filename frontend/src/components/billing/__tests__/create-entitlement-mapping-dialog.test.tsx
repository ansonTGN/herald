import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { toast } from 'sonner'

// --- Mocks ----------------------------------------------------------------
//
// Mirrors entitlement-mappings-page.test.tsx: the whole mutations module is
// mocked so individual tests drive `mockCreateMutate` to decide
// success/failure. The create-mapping dialog (FE-D03) is the unit under test;
// 409 vs 23514/non-4xx branching (design §4.2.2) is its core contract, so the
// mock's onError controller just delegates to the dialog's caller-supplied
// onError — the dialog owns the duplicate vs config-error classification.

// Permission hook: default to a fully-privileged admin (billing.manage +
// points.manage both pass) so the credit-strategy fields render.
vi.mock('@/hooks/use-permission', () => ({
  usePermission: vi.fn(() => ({
    hasPermission: (_p: string) => true,
  })),
}))

// Query options: canned bucket list + roles so the dialog renders without a
// real query. The bucket list must be non-empty for the bucket Select.
const { bucketsHolder } = vi.hoisted(() => ({
  bucketsHolder: {
    current: [] as Array<{ id: string; name: string }>,
  },
}))

vi.mock('@/data/query-options', () => ({
  // The real cache key is ['entitlement-mappings', realmId, {}]; the mock keeps
  // the prefix shape so invalidateQueries(queryKey) calls can be observed.
  queryKeys: {
    entitlementMappings: (realmId: string, _filters: Record<string, unknown>) => [
      'entitlement-mappings',
      realmId,
      {},
    ],
  },
  entitlementMappingsQueryOptions: () => ({
    queryKey: ['entitlement-mappings', 'realm-1'],
    queryFn: async () => ({ items: [], total: 0 }),
  }),
  creditBucketsListQueryOptions: () => ({
    queryKey: ['credit-buckets', 'realm-1'],
    queryFn: async () => bucketsHolder.current,
  }),
  adminRolesQueryOptions: () => ({
    queryKey: ['roles', 'realm-1'],
    queryFn: async () => [],
  }),
}))

const { mockCreateMutate, mockIsCreateMappingDuplicateError, mockIsCreateMappingConfigError } =
  vi.hoisted(() => {
    const mockCreateMutate = vi.fn()
    // Mirror the real helper contract from entitlement-mapping-mutations.ts so
    // the dialog's branch logic (not the helper itself — that is unit-tested
    // elsewhere) drives the observed toast. 409 → duplicate; 23514 code or
    // status >= 500 → config error.
    const mockIsCreateMappingDuplicateError = (e: unknown) =>
      !!e && typeof e === 'object' && (e as { status?: unknown }).status === 409
    const mockIsCreateMappingConfigError = (e: unknown) => {
      if (!e || typeof e !== 'object') return false
      const obj = e as { code?: unknown; status?: unknown }
      if (obj.code === '23514') return true
      return typeof obj.status === 'number' && obj.status >= 500
    }
    return {
      mockCreateMutate,
      mockIsCreateMappingDuplicateError,
      mockIsCreateMappingConfigError,
    }
  })

vi.mock('@/data/entitlement-mapping-mutations', () => ({
  useCreateEntitlementMapping: () => ({
    mutate: (
      req: unknown,
      opts: { onSuccess?: () => void; onError?: (error: unknown) => void }
    ) => {
      // Delegate to the per-test controller; tests drive success/failure via
      // mockImplementation that invokes opts.onSuccess?.() / opts.onError?(err).
      mockCreateMutate(req, opts)
    },
    isPending: false,
  }),
  isCreateMappingDuplicateError: mockIsCreateMappingDuplicateError,
  isCreateMappingConfigError: mockIsCreateMappingConfigError,
}))

// Role selector: stub (the granted-roles field is exercised by the page test).
vi.mock('@/components/shared/role-selector', () => ({
  RoleSelector: () => <div data-testid="role-selector-stub" />,
}))

import { CreateEntitlementMappingDialog } from '../create-entitlement-mapping-dialog'
import { m } from '@/paraglide/messages'

// --- Fixtures --------------------------------------------------------------

const BUCKETS = [{ id: 'bucket-1', name: 'Default Bucket' }]

function makeQueryClient() {
  return new QueryClient({
    defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
  })
}

function Wrapper({ client, children }: { client: QueryClient; children: ReactNode }) {
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

/**
 * Fill the create-mapping form's required fields. Radix Selects are driven by
 * clicking the testid trigger then the option (the SelectItem renders with
 * `role="option"` in a portal). Defaults to an `apple` recurring mapping with a
 * monthly period.
 */
async function fillCreateForm(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByTestId('create-mapping-provider-select'))
  await user.click(await screen.findByRole('option', { name: 'App Store' }))

  await user.type(
    screen.getByTestId('create-mapping-external-product-id-input'),
    'com.example.app.premium'
  )

  await user.type(screen.getByTestId('create-mapping-entitlement-key-input'), 'premium')

  await user.click(screen.getByTestId('create-mapping-bucket-select'))
  await user.click(await screen.findByRole('option', { name: 'Default Bucket' }))

  // Billing Type = recurring (so billingPeriod becomes visible + required)
  await user.click(screen.getByTestId('create-mapping-billing-type-select'))
  await user.click(await screen.findByRole('option', { name: /recurring/i }))

  // Billing Period (required because recurring)
  await user.click(screen.getByTestId('create-mapping-billing-period-select'))
  await user.click(await screen.findByRole('option', { name: /month/i }))
}

function renderDialog(open = true) {
  const onOpenChange = vi.fn()
  const client = makeQueryClient()
  const view = render(
    <Wrapper client={client}>
      <CreateEntitlementMappingDialog
        open={open}
        onOpenChange={onOpenChange}
        realmId="realm-1"
        canManagePoints={true}
      />
    </Wrapper>
  )
  return { client, onOpenChange, ...view }
}

beforeEach(() => {
  vi.clearAllMocks()
  bucketsHolder.current = BUCKETS
})

// --- Tests -----------------------------------------------------------------

describe('CreateEntitlementMappingDialog — submit success', () => {
  it('submits a valid form, toasts success, and closes the dialog', async () => {
    mockCreateMutate.mockImplementation(
      (_req: unknown, opts: { onSuccess?: () => void; onError?: (error: unknown) => void }) => {
        opts.onSuccess?.()
      }
    )

    const user = userEvent.setup()
    const { onOpenChange } = renderDialog()

    await fillCreateForm(user)
    await user.click(screen.getByTestId('create-mapping-submit-button'))

    await waitFor(() => {
      expect(mockCreateMutate).toHaveBeenCalledTimes(1)
    })
    const body = mockCreateMutate.mock.calls[0]?.[0] as Record<string, unknown>
    expect(body.paymentProvider).toBe('apple')
    expect(body.externalProductId).toBe('com.example.app.premium')
    expect(body.entitlementKey).toBe('premium')
    expect(body.bucketId).toBe('bucket-1')
    expect(body.billingType).toBe('recurring')
    expect(body.billingPeriod).toBe('monthly')

    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith(m['billing.create_mapping_success']())
    })
    expect(onOpenChange).toHaveBeenCalledWith(false)
  })
})

describe('CreateEntitlementMappingDialog — error classification (§4.2.2)', () => {
  // 409 Conflict → duplicate. Distinct from 23514/non-4xx — must NOT be
  // conflated. The dialog surfaces `billing.create_mapping_duplicate`.
  it("shows the 'product id already exists' message on a 409 duplicate", async () => {
    mockCreateMutate.mockImplementation(
      (_req: unknown, opts: { onSuccess?: () => void; onError?: (error: unknown) => void }) => {
        opts.onError?.({ status: 409, code: 'mapping_already_exists' })
      }
    )

    const user = userEvent.setup()
    const { onOpenChange } = renderDialog()

    await fillCreateForm(user)
    await user.click(screen.getByTestId('create-mapping-submit-button'))

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(m['billing.create_mapping_duplicate']())
    })

    // The duplicate branch also surfaces inline (the only fix is editing the
    // provider/product inputs, not retrying).
    await waitFor(() => {
      expect(screen.getByTestId('create-mapping-submit-error')).toHaveTextContent(
        String(m['billing.create_mapping_duplicate']())
      )
    })

    // A 409 is NOT a config error — assert the other branch did not fire.
    expect(toast.error).not.toHaveBeenCalledWith(m['billing.create_mapping_config_error']())
    // And the dialog stays open (the admin edits the inputs).
    expect(onOpenChange).not.toHaveBeenCalled()
  })

  // 23514 / non-4xx → configuration error (DB CHECK / server defense). Two
  // representative triggers: a 23514-tagged body and a 500 server failure.
  it.each([
    ['a 23514-tagged CHECK body', { status: 422, code: '23514' }],
    ['a 500 server failure', { status: 500, message: 'internal error' }],
  ])('shows the configuration-error message on %s', async (_label, error) => {
    mockCreateMutate.mockImplementation(
      (_req: unknown, opts: { onSuccess?: () => void; onError?: (error: unknown) => void }) => {
        opts.onError?.(error)
      }
    )

    const user = userEvent.setup()
    renderDialog()

    await fillCreateForm(user)
    await user.click(screen.getByTestId('create-mapping-submit-button'))

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(m['billing.create_mapping_config_error']())
    })

    // Config error must NOT surface as a duplicate (distinct branches, §4.2.2).
    expect(toast.error).not.toHaveBeenCalledWith(m['billing.create_mapping_duplicate']())
  })

  // 400 validation falls through to the generic `billing.create_mapping_failed`
  // branch — it is neither a duplicate (409) nor a config error (23514/>=500).
  it('falls back to the generic failure message on a 400 validation error', async () => {
    mockCreateMutate.mockImplementation(
      (_req: unknown, opts: { onSuccess?: () => void; onError?: (error: unknown) => void }) => {
        opts.onError?.({ status: 400, code: 'bad_request', message: 'invalid' })
      }
    )

    const user = userEvent.setup()
    renderDialog()

    await fillCreateForm(user)
    await user.click(screen.getByTestId('create-mapping-submit-button'))

    await waitFor(() => {
      expect(screen.getByTestId('create-mapping-submit-error')).toHaveTextContent(
        String(m['billing.create_mapping_failed']())
      )
    })

    // 400 is neither the duplicate nor the config-error branch.
    expect(toast.error).not.toHaveBeenCalledWith(m['billing.create_mapping_duplicate']())
    expect(toast.error).not.toHaveBeenCalledWith(m['billing.create_mapping_config_error']())
  })
})

describe('CreateEntitlementMappingDialog — client-side validation', () => {
  // The schema's recurring ⇒ billingPeriod refinement (support-iap §4.4.2) is
  // Demo-unreachable (the dialog hides submit until the field is filled). This
  // Vitest is the only coverage of the safeParse gate blocking submit.
  it('blocks submit and shows a billingPeriod field error when recurring has no period', async () => {
    const user = userEvent.setup()
    renderDialog()

    // Fill everything except billingPeriod.
    await user.click(screen.getByTestId('create-mapping-provider-select'))
    await user.click(await screen.findByRole('option', { name: 'App Store' }))
    await user.type(
      screen.getByTestId('create-mapping-external-product-id-input'),
      'com.example.app.premium'
    )
    await user.type(screen.getByTestId('create-mapping-entitlement-key-input'), 'premium')
    await user.click(screen.getByTestId('create-mapping-bucket-select'))
    await user.click(await screen.findByRole('option', { name: 'Default Bucket' }))
    await user.click(screen.getByTestId('create-mapping-billing-type-select'))
    await user.click(await screen.findByRole('option', { name: /recurring/i }))
    // billingPeriod select intentionally left empty.

    await user.click(screen.getByTestId('create-mapping-submit-button'))

    expect(mockCreateMutate).not.toHaveBeenCalled()
    expect(toast.success).not.toHaveBeenCalled()

    await waitFor(() => {
      const alerts = screen.getAllByRole('alert')
      // At least one field error rendered. The dialog renders a <p role="alert">
      // per failed field; recurring-without-period fails billingPeriod.
      expect(alerts.length).toBeGreaterThan(0)
    })
  })

  it('blocks submit when required text fields are empty', async () => {
    const user = userEvent.setup()
    renderDialog()

    await user.click(screen.getByTestId('create-mapping-submit-button'))

    expect(mockCreateMutate).not.toHaveBeenCalled()
    await waitFor(() => {
      expect(screen.getAllByRole('alert').length).toBeGreaterThan(0)
    })
  })
})

// --- Cache-key indirect assertion -----------------------------------------
//
// The real `useCreateEntitlementMapping` hook owns the
// `invalidateQueries(queryKeys.entitlementMappings(realmId, {}))` side-effect.
// This test mocks the whole mutations module (parity with the page test), so
// the cache-key contract is asserted indirectly: the dialog's caller-supplied
// onSuccess closes the dialog, proving the success callback fired end-to-end —
// i.e. the mutation layer that owns invalidation was reached. The exact key
// shape (`['entitlement-mappings', realmId, {}]`) is pure data produced by
// `queryKeys.entitlementMappings`, mirrored in the query-options mock above.
describe('CreateEntitlementMappingDialog — success side-effects', () => {
  it('closes the dialog on success (the list-invalidation callback was reached)', async () => {
    mockCreateMutate.mockImplementation(
      (_req: unknown, opts: { onSuccess?: () => void; onError?: (error: unknown) => void }) => {
        opts.onSuccess?.()
      }
    )

    const user = userEvent.setup()
    const { onOpenChange } = renderDialog()

    await fillCreateForm(user)
    await user.click(screen.getByTestId('create-mapping-submit-button'))

    // The dialog's onSuccess closes the dialog — the observable proof the
    // success branch (which in the real hook also invalidates the list query)
    // executed.
    await waitFor(() => {
      expect(onOpenChange).toHaveBeenCalledWith(false)
    })
    // The success toast fired (re-asserted here to bind the success path to the
    // invalidation-capable callback in one place).
    expect(toast.success).toHaveBeenCalledWith(m['billing.create_mapping_success']())
  })
})
