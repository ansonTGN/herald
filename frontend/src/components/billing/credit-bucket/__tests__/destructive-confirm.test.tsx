import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { DeleteBucketConfirmDialog } from '../delete-bucket-confirm-dialog'
import { CreditBucketEditor } from '../credit-bucket-editor'
import type { BucketDetailResponse } from '@/lib/api-generated'

vi.mock('@/data/query-options', () => ({
  clientAppsQueryOptions: () => ({
    queryKey: ['client-apps'],
    queryFn: async () => ({ items: [{ id: 'app-1', name: 'App', clientId: 'app' }] }),
  }),
}))

vi.mock('@/data/credit-bucket-mutations', () => ({
  useCreateCreditBucket: () => ({ mutateAsync: vi.fn(), isPending: false }),
  useUpdateCreditBucket: () => ({ mutateAsync: vi.fn(), isPending: false }),
}))

function renderWithQuery(ui: React.ReactNode) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>)
}

describe('Credit Bucket destructive and reference states', () => {
  it('shows why a bucket in use cannot be deleted', () => {
    renderWithQuery(
      <DeleteBucketConfirmDialog
        open
        onOpenChange={vi.fn()}
        onConfirm={vi.fn()}
        bucketName="Promo Pool"
        inUseError={{
          code: 'bucket_in_use',
          activeSubscriptions: 2,
          holdersWithBalance: 5,
        }}
      />
    )

    expect(screen.getByTestId('delete-bucket-error-message')).toHaveTextContent('2')
    expect(screen.getByTestId('delete-bucket-error-message')).toHaveTextContent('5')
    expect(screen.queryByTestId('delete-bucket-confirm-button')).not.toBeInTheDocument()
  })

  it('shows active and disabled distribution-rule references without a registration switch', async () => {
    const bucket: BucketDetailResponse = {
      id: 'bucket-1',
      bucketKey: 'general',
      name: 'General',
      description: null,
      displayOrder: 0,
      enabled: true,
      clientApps: [{ id: 'app-1' }],
      ruleReferences: [
        {
          ruleId: 'mapping-rule',
          ownerType: 'entitlement_mapping',
          entitlementMappingId: 'mapping-1',
          triggerSources: ['topup'],
          enabled: true,
        },
        {
          ruleId: 'registration-rule',
          ownerType: 'realm_registration',
          triggerSources: ['registration'],
          enabled: false,
        },
      ],
    }

    renderWithQuery(
      <CreditBucketEditor realmId="realm-1" bucket={bucket} formKey={bucket.id} onSaved={vi.fn()} />
    )

    const references = await screen.findByTestId('credit-bucket-rule-references')
    expect(references).toHaveTextContent('mapping-rule')
    expect(references).toHaveTextContent('registration-rule')
    expect(references).toHaveTextContent('disabled')
    expect(screen.queryByRole('switch', { name: /registration/i })).not.toBeInTheDocument()
  })
})
