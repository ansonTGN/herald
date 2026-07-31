import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return {
    ...actual,
    createFileRoute: () => (config: Record<string, unknown>) => ({
      ...config,
      useParams: () => ({ realmId: 'realm-1' }),
    }),
  }
})

const { updateRules, permissions } = vi.hoisted(() => ({
  updateRules: vi.fn(),
  permissions: { current: ['points.view', 'points.manage'] as string[] },
}))

vi.mock('@/hooks/use-auth', () => ({
  useAuth: () => ({ permissions: permissions.current }),
}))

vi.mock('@/data/query-options', () => ({
  queryKeys: {
    pointsDefaultConfig: (realmId: string) => ['registration-rules', realmId],
  },
  pointsDefaultConfigQueryOptions: (realmId: string) => ({
    queryKey: ['registration-rules', realmId],
    queryFn: async () => ({
      realmId,
      rules: [
        {
          id: 'registration-rule',
          bucketId: 'bucket-a',
          triggerSources: ['registration'],
          grantMode: 'fixed',
          pointsAmount: 50,
          validityDays: 0,
          enabled: true,
          displayOrder: 0,
        },
        {
          id: 'periodic-rule',
          bucketId: 'bucket-b',
          triggerSources: ['free_periodic_grant'],
          grantMode: 'quota',
          quotaWindows: [{ windowSeconds: 3600, limit: 10 }],
          enabled: true,
          displayOrder: 0,
        },
      ],
    }),
  }),
  creditBucketsListQueryOptions: (realmId: string) => ({
    queryKey: ['credit-buckets', realmId],
    queryFn: async () => [
      {
        id: 'bucket-a',
        name: 'General',
        bucketKey: 'general',
        displayOrder: 0,
        enabled: true,
        coveredClientAppCount: 1,
        ruleReferenceCount: 1,
      },
      {
        id: 'bucket-b',
        name: 'Images',
        bucketKey: 'images',
        displayOrder: 1,
        enabled: true,
        coveredClientAppCount: 1,
        ruleReferenceCount: 1,
      },
    ],
  }),
  updatePointsDefaultConfigMutation: updateRules,
}))

import { RealmConfigPage } from '@/routes/$realmId/manage/points/default-config'

function renderPage() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <RealmConfigPage />
    </QueryClientProvider>
  )
}

describe('registration distribution rules', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    permissions.current = ['points.view', 'points.manage']
    updateRules.mockResolvedValue({ realmId: 'realm-1', rules: [] })
  })

  it('renders registration fixed grants and free-periodic quota as separate rules', async () => {
    renderPage()
    expect(await screen.findByTestId('point-rule-registration-rule')).toBeInTheDocument()
    expect(screen.getByTestId('point-rule-periodic-rule')).toBeInTheDocument()
    expect(screen.getByTestId('point-rule-quota-periodic-rule-editor')).toBeInTheDocument()
  })

  it('submits the complete rule set without scalar default-config fields', async () => {
    const user = userEvent.setup()
    renderPage()
    await user.click(await screen.findByTestId('registration-rules-save'))

    await waitFor(() => expect(updateRules).toHaveBeenCalledOnce())
    const [, body] = updateRules.mock.calls[0] ?? []
    expect(body.rules).toHaveLength(2)
    expect(body.rules).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: 'registration-rule', triggerSources: ['registration'] }),
        expect.objectContaining({
          id: 'periodic-rule',
          triggerSources: ['free_periodic_grant'],
          grantMode: 'quota',
        }),
      ])
    )
    expect(body).not.toHaveProperty('registrationBonusPoints')
  })

  it('allows points.view users to inspect rules without edit controls', async () => {
    permissions.current = ['points.view']
    renderPage()
    expect(await screen.findByTestId('point-rule-registration-rule')).toBeInTheDocument()
    expect(screen.queryByTestId('registration-rules-save')).not.toBeInTheDocument()
    expect(screen.getAllByTestId('point-rule-add')[0]).toBeDisabled()
  })
})
