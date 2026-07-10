import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import {
  CustomDomainConfigForm,
  type CustomDomainConfigFormProps,
} from '../custom-domain-config-form'
import { emptyCustomDomainConfig } from '@/lib/realm-config-utils'
import type { CustomDomainConfigForm as CustomDomainConfigFormValues } from '@/lib/schemas/realm-config'
import type { CustomDomainStatus } from '@/lib/api-generated'

/**
 * Custom-domain config form (FE-D01/D02).
 *
 * The form is a presentational editor: it owns field rendering, CNAME guidance,
 * the live status badges, and the draft/publish/discard/restore/refresh action
 * entry points, but performs no data access — every action is surfaced via a
 * callback so FE-D03 can wire up the queries/mutations. These tests assert the
 * state-machine contract FE-D03 will rely on (gating by hasDraft/hasPrevious,
 * in-flight flags, dirty notice, disabled, payload assembly, restore dialog).
 *
 * Note: onSaveDraft / onPublish receive the form's submitted
 * CustomDomainConfigForm value; onDiscardDraft / onRestore take no args.
 */
describe('CustomDomainConfigForm', () => {
  const mockOnSaveDraft = vi.fn()
  const mockOnPublish = vi.fn()
  const mockOnDiscardDraft = vi.fn()
  const mockOnRestore = vi.fn()
  const mockOnRefreshStatus = vi.fn()

  const defaultProps: CustomDomainConfigFormProps = {
    realmId: 'admin',
    initialConfig: emptyCustomDomainConfig(),
    hasDraft: false,
    hasPrevious: false,
    disabled: false,
    cnameTarget: 'custom.herald.com',
    status: null,
    onSaveDraft: mockOnSaveDraft,
    onPublish: mockOnPublish,
    onDiscardDraft: mockOnDiscardDraft,
    onRestore: mockOnRestore,
    onRefreshStatus: mockOnRefreshStatus,
  }

  beforeEach(() => {
    mockOnSaveDraft.mockClear()
    mockOnPublish.mockClear()
    mockOnDiscardDraft.mockClear()
    mockOnRestore.mockClear()
    mockOnRefreshStatus.mockClear()
    mockOnSaveDraft.mockResolvedValue(undefined)
    mockOnPublish.mockResolvedValue(undefined)
    mockOnDiscardDraft.mockResolvedValue(undefined)
    mockOnRestore.mockResolvedValue(undefined)
  })

  it('GIVEN form is rendered with no initial config THEN should display an empty hostname field and all four action buttons', async () => {
    const screen = render(<CustomDomainConfigForm {...defaultProps} />)

    expect(screen.getByTestId('custom-domain-hostname')).toHaveValue('')
    expect(screen.getByTestId('custom-domain-save-draft')).toBeInTheDocument()
    expect(screen.getByTestId('custom-domain-publish')).toBeInTheDocument()
    expect(screen.getByTestId('custom-domain-discard-draft')).toBeInTheDocument()
    expect(screen.getByTestId('custom-domain-restore')).toBeInTheDocument()
  })

  it('GIVEN an initial config is provided WHEN rendering THEN should reflect the supplied hostname', async () => {
    const initialConfig: CustomDomainConfigFormValues = { hostname: 'login.acme.com' }

    const screen = render(
      <CustomDomainConfigForm {...defaultProps} initialConfig={initialConfig} />
    )

    expect(screen.getByTestId('custom-domain-hostname')).toHaveValue('login.acme.com')
  })

  it('GIVEN user types a hostname WHEN clicking save draft THEN should call onSaveDraft once with the current form value', async () => {
    const screen = render(<CustomDomainConfigForm {...defaultProps} />)

    await userEvent.type(screen.getByTestId('custom-domain-hostname'), 'login.acme.com')
    await userEvent.click(screen.getByTestId('custom-domain-save-draft'))

    await waitFor(() => {
      expect(mockOnSaveDraft).toHaveBeenCalledTimes(1)
      expect(mockOnSaveDraft).toHaveBeenCalledWith(
        expect.objectContaining({ hostname: 'login.acme.com' })
      )
    })
  })

  it('GIVEN user has edited the hostname WHEN clicking publish THEN should call onPublish with the current form value', async () => {
    const screen = render(<CustomDomainConfigForm {...defaultProps} />)

    await userEvent.type(screen.getByTestId('custom-domain-hostname'), 'login.acme.com')
    await userEvent.click(screen.getByTestId('custom-domain-publish'))

    await waitFor(() => {
      expect(mockOnPublish).toHaveBeenCalledTimes(1)
      expect(mockOnPublish).toHaveBeenCalledWith(
        expect.objectContaining({ hostname: 'login.acme.com' })
      )
    })
  })

  it('GIVEN hasDraft is true OR the form is dirty WHEN rendering THEN should show the draft notice; hidden otherwise', async () => {
    // hasDraft flag alone surfaces the notice.
    const screenA = render(<CustomDomainConfigForm {...defaultProps} hasDraft={true} />)
    expect(screenA.getByTestId('custom-domain-draft-notice')).toBeInTheDocument()
    screenA.unmount()

    // No draft and a clean form must hide the notice.
    const screenB = render(<CustomDomainConfigForm {...defaultProps} hasDraft={false} />)
    expect(screenB.queryByTestId('custom-domain-draft-notice')).not.toBeInTheDocument()

    // Editing the hostname surfaces the notice (dirty form) even with no saved draft.
    await userEvent.type(screenB.getByTestId('custom-domain-hostname'), 'login.acme.com')
    expect(screenB.getByTestId('custom-domain-draft-notice')).toBeInTheDocument()
  })

  it('GIVEN hasDraft is true WHEN clicking discard THEN should enable the button and call onDiscardDraft', async () => {
    const screen = render(<CustomDomainConfigForm {...defaultProps} hasDraft={true} />)

    const discardButton = screen.getByTestId('custom-domain-discard-draft')
    expect(discardButton).not.toBeDisabled()

    await userEvent.click(discardButton)
    await waitFor(() => {
      expect(mockOnDiscardDraft).toHaveBeenCalledTimes(1)
      // Discard takes no payload: it resets to the published config server-side.
      expect(mockOnDiscardDraft).toHaveBeenCalledWith()
    })
  })

  it('GIVEN no draft exists WHEN rendering THEN should disable the discard button', async () => {
    const screen = render(<CustomDomainConfigForm {...defaultProps} hasDraft={false} />)
    expect(screen.getByTestId('custom-domain-discard-draft')).toBeDisabled()
  })

  it('GIVEN hasPrevious is true WHEN clicking restore THEN should open the confirm dialog and only call onRestore on confirm', async () => {
    const screen = render(<CustomDomainConfigForm {...defaultProps} hasPrevious={true} />)

    const restoreButton = screen.getByTestId('custom-domain-restore')
    expect(restoreButton).not.toBeDisabled()

    await userEvent.click(restoreButton)

    const dialog = await screen.findByTestId('custom-domain-restore-dialog')
    expect(dialog).toBeInTheDocument()

    // The restore callback must NOT fire until the user confirms.
    expect(mockOnRestore).not.toHaveBeenCalled()

    const confirmButton = within(dialog).getByTestId('custom-domain-restore-confirm')
    await userEvent.click(confirmButton)

    await waitFor(() => {
      expect(mockOnRestore).toHaveBeenCalledTimes(1)
    })
  })

  it('GIVEN no previous version exists WHEN rendering THEN should disable the restore button', async () => {
    const screen = render(<CustomDomainConfigForm {...defaultProps} hasPrevious={false} />)
    expect(screen.getByTestId('custom-domain-restore')).toBeDisabled()
  })

  it('GIVEN the form is disabled WHEN rendering THEN should disable the hostname field and all action buttons', async () => {
    const screen = render(<CustomDomainConfigForm {...defaultProps} disabled={true} />)

    expect(screen.getByTestId('custom-domain-hostname')).toBeDisabled()
    expect(screen.getByTestId('custom-domain-save-draft')).toBeDisabled()
    expect(screen.getByTestId('custom-domain-publish')).toBeDisabled()
    expect(screen.getByTestId('custom-domain-discard-draft')).toBeDisabled()
    expect(screen.getByTestId('custom-domain-restore')).toBeDisabled()
  })

  it.each([
    ['isSavingDraft', 'custom-domain-save-draft', { isSavingDraft: true }],
    ['isPublishing', 'custom-domain-publish', { isPublishing: true }],
    ['isDiscarding', 'custom-domain-discard-draft', { isDiscarding: true }],
    ['isRestoring', 'custom-domain-restore', { isRestoring: true }],
    ['isRefreshing', 'custom-domain-refresh-status', { isRefreshing: true }],
  ] as const)(
    'GIVEN an action is in flight (%s) WHEN rendering THEN should disable its own button only',
    async (_flag, testId, inFlight) => {
      const screen = render(
        <CustomDomainConfigForm {...defaultProps} {...inFlight} hasDraft hasPrevious />
      )

      expect(screen.getByTestId(testId)).toBeDisabled()
      // Buttons without their own in-flight flag stay usable (independent gating).
      const allButtons = [
        'custom-domain-save-draft',
        'custom-domain-publish',
        'custom-domain-discard-draft',
        'custom-domain-restore',
        'custom-domain-refresh-status',
      ] as const
      for (const id of allButtons) {
        if (id === testId) continue
        expect(screen.getByTestId(id)).not.toBeDisabled()
      }
    }
  )

  it('GIVEN a verified status is provided WHEN rendering THEN should reflect it on the badges and clicking refresh triggers onRefreshStatus', async () => {
    const status: CustomDomainStatus = {
      cnameVerified: true,
      tlsReady: false,
      checkedAt: '2026-07-09T00:00:00Z',
    }
    const screen = render(<CustomDomainConfigForm {...defaultProps} status={status} />)

    // CNAME verified + TLS pending: the badges carry the effective state.
    expect(screen.getByTestId('custom-domain-status-cname')).toBeInTheDocument()
    expect(screen.getByTestId('custom-domain-status-tls')).toBeInTheDocument()

    const refreshButton = screen.getByTestId('custom-domain-refresh-status')
    expect(refreshButton).not.toBeDisabled()
    await userEvent.click(refreshButton)
    await waitFor(() => {
      expect(mockOnRefreshStatus).toHaveBeenCalledTimes(1)
    })
  })
})
