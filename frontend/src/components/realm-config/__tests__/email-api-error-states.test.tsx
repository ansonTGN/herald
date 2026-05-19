import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import { userEvent } from '@testing-library/user-event'
import { QueryClientProvider, useQuery } from '@tanstack/react-query'
import { http, HttpResponse } from 'msw'
import React from 'react'
import { EmailConfigForm } from '../email-config-form'
import { server } from '@/test/mocks/server'
import { emailStatusQueryOptions } from '@/data/query-options'
import { createTestQueryClient, renderWithProviders } from '@/test/utils/render'
import { getErrorMessage } from '@/lib/error-utils'
import type { EmailConfigForm as EmailConfigFormValues } from '@/lib/schemas/realm-config'

const REALM_ID = 'test-realm'

function EmailConfigFormWithStatusQuery({
  onSave,
  initialConfig,
  disabled,
  isLoading,
}: {
  onSave?: (config: EmailConfigFormValues) => Promise<void>
  initialConfig?: EmailConfigFormValues
  disabled?: boolean
  isLoading?: boolean
}) {
  const queryClient = createTestQueryClient()
  return (
    <QueryClientProvider client={queryClient}>
      <InnerFormWrapper
        onSave={onSave}
        initialConfig={initialConfig}
        disabled={disabled}
        isLoading={isLoading}
      />
    </QueryClientProvider>
  )
}

function InnerFormWrapper({
  onSave,
  initialConfig,
  disabled,
  isLoading,
}: {
  onSave?: (config: EmailConfigFormValues) => Promise<void>
  initialConfig?: EmailConfigFormValues
  disabled?: boolean
  isLoading?: boolean
}) {
  const { data: emailStatusData, error: emailStatusQueryError } = useQuery({
    ...emailStatusQueryOptions(REALM_ID),
    retry: false,
  })

  return (
    <EmailConfigForm
      realmId={REALM_ID}
      onSave={onSave ?? vi.fn().mockResolvedValue(undefined)}
      initialConfig={initialConfig}
      isLoading={isLoading}
      disabled={disabled}
      emailStatus={emailStatusData ?? null}
      emailStatusError={emailStatusQueryError ? getErrorMessage(emailStatusQueryError) : null}
    />
  )
}

describe('Email API error states', () => {
  const mockOnSave = vi.fn()

  beforeEach(() => {
    mockOnSave.mockReset()
  })

  describe('email status endpoint returns 500', () => {
    it('shows email-status-error and keeps form usable', async () => {
      server.use(
        http.get('*/api/configs/:realmId/email/status', () =>
          HttpResponse.json({ code: 500, message: 'Internal server error' }, { status: 500 })
        )
      )

      render(<EmailConfigFormWithStatusQuery onSave={mockOnSave} />)

      const statusError = await screen.findByTestId('email-status-error')
      expect(statusError).toBeInTheDocument()
      expect(statusError.textContent).toContain('Internal server error')

      // Form remains usable
      expect(screen.getByTestId('email-from-address-input')).toBeInTheDocument()
      expect(screen.getByTestId('email-save-button')).toBeInTheDocument()
      expect(screen.getByTestId('email-test-button')).toBeInTheDocument()
    })
  })

  describe('test email endpoint returns 500', () => {
    it('shows email-test-error and keeps form usable', async () => {
      server.use(
        http.post(`http://localhost:3000/api/configs/${REALM_ID}/email/test`, () =>
          HttpResponse.json({ code: 500, message: 'Failed to send test email' }, { status: 500 })
        )
      )

      renderWithProviders(<EmailConfigForm realmId={REALM_ID} onSave={mockOnSave} />)

      // Fill in a recipient and trigger test email
      await userEvent.type(screen.getByTestId('email-test-recipient-input'), 'test@example.com')
      await userEvent.click(screen.getByTestId('email-test-button'))

      const testError = await screen.findByTestId('email-test-error')
      expect(testError).toBeInTheDocument()

      // Form remains usable after error
      expect(screen.getByTestId('email-from-address-input')).toBeInTheDocument()
      expect(screen.getByTestId('email-save-button')).not.toBeDisabled()
    })
  })

  describe('test email endpoint returns 400', () => {
    it('shows email-test-error with server message when email not configured', async () => {
      server.use(
        http.post(`http://localhost:3000/api/configs/${REALM_ID}/email/test`, () =>
          HttpResponse.json(
            {
              code: 400,
              message: 'Email is not configured for this realm',
            },
            { status: 400 }
          )
        )
      )

      renderWithProviders(<EmailConfigForm realmId={REALM_ID} onSave={mockOnSave} />)

      await userEvent.type(screen.getByTestId('email-test-recipient-input'), 'test@example.com')
      await userEvent.click(screen.getByTestId('email-test-button'))

      const testError = await screen.findByTestId('email-test-error')
      expect(testError).toBeInTheDocument()
      expect(testError.textContent).toContain('Email is not configured for this realm')
    })
  })

  describe('network error on save', () => {
    it('shows email-save-error and allows retry', async () => {
      server.use(
        http.post(`http://localhost:3000/api/configs/${REALM_ID}/batch`, () => HttpResponse.error())
      )

      // onSave calls batchUpsertRealmConfigs which will trigger MSW network error
      const failingSave = vi.fn().mockRejectedValue(new Error('Network error'))

      renderWithProviders(<EmailConfigForm realmId={REALM_ID} onSave={failingSave} />)

      // Fill required field and submit
      await userEvent.type(screen.getByTestId('email-from-address-input'), 'noreply@example.com')
      await userEvent.click(screen.getByTestId('email-save-button'))

      const saveError = await screen.findByTestId('email-save-error')
      expect(saveError).toBeInTheDocument()
      expect(saveError.textContent).toContain('Network error')

      // Verify no crash and user can retry
      expect(screen.getByTestId('email-save-button')).toBeInTheDocument()
      expect(screen.getByTestId('email-from-address-input')).toBeInTheDocument()
    })
  })
})
