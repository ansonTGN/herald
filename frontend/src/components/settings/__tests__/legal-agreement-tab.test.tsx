import { describe, it, expect, vi, beforeEach } from 'vitest'
import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { http, HttpResponse } from 'msw'
import { server } from '@/test/mocks/server'
import { renderWithProviders } from '@/test/utils/render'
import { LegalAgreementTab } from '../LegalAgreementTab'
import type { AdminAgreementView } from '@/lib/api-generated'

const realmId = 'test-realm'

function makeVersionSummary(overrides?: Record<string, unknown>) {
  return {
    version_id: 'version-001',
    version_no: 1,
    effective_at: '2026-06-30T00:00:00Z',
    source: 'default' as const,
    version_label: null,
    ...overrides,
  }
}

function makeAgreementView(overrides?: Partial<AdminAgreementView>): AdminAgreementView {
  return {
    agreement_type: 'terms_of_service',
    source: 'default',
    current_version: makeVersionSummary({
      agreement_type: 'terms_of_service',
      version_id: 'tos-v1',
      version_no: 1,
    }) as AdminAgreementView['current_version'],
    history: [],
    ...overrides,
  }
}

function setupAdminAgreementsHandler(response: { agreements: AdminAgreementView[] }, status = 200) {
  server.use(
    http.get(`http://localhost:3000/api/legal/admin/${realmId}/agreements`, () =>
      HttpResponse.json(response, { status })
    )
  )
}

describe('LegalAgreementTab', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('shows loading state while fetching agreements', () => {
    setupAdminAgreementsHandler({ agreements: [] })
    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage={false} />)

    expect(screen.getByTestId('legal-agreements-loading')).toBeInTheDocument()
  })

  it('shows error state and allows retry', async () => {
    setupAdminAgreementsHandler({ agreements: [] }, 500)
    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage={false} />)

    const errorAlert = await screen.findByTestId('legal-agreements-error', {}, { timeout: 3000 })
    expect(errorAlert).toBeInTheDocument()

    const retryButton = screen.getByTestId('legal-agreements-retry')
    expect(retryButton).toBeInTheDocument()
  })

  it('shows empty state when no agreements exist', async () => {
    setupAdminAgreementsHandler({ agreements: [] })
    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage={false} />)

    expect(await screen.findByTestId('legal-agreements-empty')).toBeInTheDocument()
  })

  it('renders view-only notice and hides controls without manage permission', async () => {
    setupAdminAgreementsHandler({
      agreements: [
        makeAgreementView({
          agreement_type: 'terms_of_service',
          source: 'custom',
        }),
        makeAgreementView({
          agreement_type: 'privacy_policy',
          source: 'default',
          current_version: makeVersionSummary({
            agreement_type: 'privacy_policy',
            version_id: 'privacy-v1',
            version_no: 1,
          }) as AdminAgreementView['current_version'],
        }),
      ],
    })

    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage={false} />)

    expect(await screen.findByTestId('legal-agreements-view-only')).toBeInTheDocument()
    expect(screen.getByTestId('legal-agreement-view-only-terms_of_service')).toBeInTheDocument()
    expect(screen.queryByTestId('legal-publish-button-terms_of_service')).not.toBeInTheDocument()
    expect(screen.queryByTestId('legal-revert-button-terms_of_service')).not.toBeInTheDocument()
  })

  it('renders publish controls and hides revert for default source with manage permission', async () => {
    setupAdminAgreementsHandler({
      agreements: [
        makeAgreementView({
          agreement_type: 'terms_of_service',
          source: 'default',
        }),
      ],
    })

    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage />)

    expect(await screen.findByTestId('legal-publish-button-terms_of_service')).toBeInTheDocument()
    expect(screen.queryByTestId('legal-revert-button-terms_of_service')).not.toBeInTheDocument()
    expect(screen.getByTestId('legal-history-empty-terms_of_service')).toBeInTheDocument()
  })

  it('renders version history entries with source badges', async () => {
    setupAdminAgreementsHandler({
      agreements: [
        makeAgreementView({
          agreement_type: 'terms_of_service',
          source: 'custom',
          history: [
            makeVersionSummary({
              version_id: 'tos-v1',
              version_no: 1,
              source: 'default',
            }),
            makeVersionSummary({
              version_id: 'tos-v2',
              version_no: 2,
              source: 'custom',
              version_label: 'June update',
            }),
          ],
        }),
      ],
    })

    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage={false} />)

    await screen.findByTestId('legal-history-table-terms_of_service')
    expect(screen.getByTestId('legal-history-row-terms_of_service-tos-v1')).toBeInTheDocument()
    expect(screen.getByTestId('legal-history-row-terms_of_service-tos-v2')).toBeInTheDocument()
    expect(screen.getAllByTestId('source-badge-default')).toHaveLength(1)
    expect(screen.getAllByTestId('source-badge-custom')).toHaveLength(2)
  })

  it('validates that at least one locale content is provided before publishing', async () => {
    setupAdminAgreementsHandler({
      agreements: [makeAgreementView()],
    })

    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage />)
    const user = userEvent.setup()

    const publishButton = await screen.findByTestId('legal-publish-button-terms_of_service')
    await user.click(publishButton)

    expect(await screen.findByText(/English content is required/i)).toBeInTheDocument()
  })

  it('publishes a custom version and invalidates the admin agreements query', async () => {
    let listCalls = 0
    let requestBody: unknown

    server.use(
      http.get(`http://localhost:3000/api/legal/admin/${realmId}/agreements`, () => {
        listCalls += 1
        return HttpResponse.json({
          agreements: [
            makeAgreementView({
              agreement_type: 'terms_of_service',
              source: 'default',
            }),
          ],
        })
      }),
      http.put(
        `http://localhost:3000/api/legal/admin/${realmId}/agreements/terms_of_service`,
        async ({ request }) => {
          requestBody = await request.json()
          return HttpResponse.json({
            version_id: 'tos-v2',
            version_no: 2,
            effective_at: '2026-07-01T00:00:00Z',
          })
        }
      )
    )

    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage />)
    const user = userEvent.setup()

    await screen.findByTestId('legal-publish-button-terms_of_service')

    await user.type(
      screen.getByTestId('legal-version-label-input-terms_of_service'),
      'Summer update'
    )
    await user.type(screen.getByTestId('legal-content-en-input-terms_of_service'), 'Updated terms')
    await user.click(screen.getByTestId('legal-publish-button-terms_of_service'))

    await waitFor(() => {
      expect(requestBody).toEqual({
        content: { en: 'Updated terms' },
        version_label: 'Summer update',
      })
    })

    await waitFor(() => {
      expect(listCalls).toBeGreaterThanOrEqual(2)
    })
  })

  it('reverts a custom agreement to platform default after confirmation', async () => {
    let listCalls = 0
    let revertCalled = false

    server.use(
      http.get(`http://localhost:3000/api/legal/admin/${realmId}/agreements`, () => {
        listCalls += 1
        return HttpResponse.json({
          agreements: [
            makeAgreementView({
              agreement_type: 'terms_of_service',
              source: 'custom',
            }),
          ],
        })
      }),
      http.delete(
        `http://localhost:3000/api/legal/admin/${realmId}/agreements/terms_of_service/custom`,
        () => {
          revertCalled = true
          return HttpResponse.json({
            version_id: 'tos-v3',
            version_no: 3,
            effective_at: '2026-07-02T00:00:00Z',
          })
        }
      )
    )

    renderWithProviders(<LegalAgreementTab realmId={realmId} canManage />)
    const user = userEvent.setup()

    await screen.findByTestId('legal-revert-button-terms_of_service')
    await user.click(screen.getByTestId('legal-revert-button-terms_of_service'))

    expect(
      await screen.findByTestId('legal-revert-dialog-title-terms_of_service')
    ).toBeInTheDocument()

    await user.click(screen.getByTestId('legal-revert-confirm-terms_of_service'))

    await waitFor(() => {
      expect(revertCalled).toBe(true)
    })

    await waitFor(() => {
      expect(listCalls).toBeGreaterThanOrEqual(2)
    })
  })
})
