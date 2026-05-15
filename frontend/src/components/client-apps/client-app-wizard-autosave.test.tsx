/**
 * @vitest-environment jsdom
 */

import React from 'react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ClientAppWizard } from './client-app-wizard'
import { MemoryRouter } from '@tanstack/react-router'

// Mock API calls
vi.mock('@/lib/api-generated', () => ({
  createClientApp: vi.fn(),
  updateClientApp: vi.fn(),
}))

describe('ClientAppWizard Auto-save Integration', () => {
  let queryClient: QueryClient

  beforeEach(() => {
    localStorage.clear()
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    })
  })

  const renderWithProviders = (component: React.ReactElement) => {
    return render(
      <QueryClientProvider client={queryClient}>
        <MemoryRouter initialEntries={['/realm1/manage/client-apps/create']}>
          {component}
        </MemoryRouter>
      </QueryClientProvider>
    )
  }

  it('should detect and offer to restore existing draft', async () => {
    // Save a draft first
    const draftData = {
      data: {
        name: 'Draft App',
        description: 'Draft Description',
        appType: 'WEB' as const,
        clientType: 'CONFIDENTIAL' as const,
        redirectUris: [],
        postLogoutUris: [],
        webOrigins: [],
        sessionTtlSeconds: 3600,
      },
      timestamp: Date.now(),
      version: '1.0',
    }
    localStorage.setItem('client-app-draft-realm1-create-new', JSON.stringify(draftData))

    renderWithProviders(<ClientAppWizard mode="create" realmId="realm1" />)

    // Should show restore dialog
    await waitFor(() => {
      expect(screen.getByTestId('draft-restore-dialog')).toBeInTheDocument()
    })
  })

  it('should clear draft when canceling', async () => {
    const user = userEvent.setup()

    // Create a draft
    const draftData = {
      data: {
        name: 'Test',
        description: '',
        appType: 'WEB' as const,
        clientType: 'CONFIDENTIAL' as const,
        redirectUris: [],
        postLogoutUris: [],
        webOrigins: [],
        sessionTtlSeconds: 3600,
      },
      timestamp: Date.now(),
      version: '1.0',
    }
    localStorage.setItem('client-app-draft-realm1-create-new', JSON.stringify(draftData))

    renderWithProviders(<ClientAppWizard mode="create" realmId="realm1" />)

    const cancelButton = screen.getByTestId('cancel-button')
    await user.click(cancelButton)

    // Draft should be cleared after cancel
    expect(localStorage.getItem('client-app-draft-realm1-create-new')).toBeNull()
  })
})
