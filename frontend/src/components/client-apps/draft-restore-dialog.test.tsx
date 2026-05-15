/**
 * @vitest-environment jsdom
 */

import React from 'react'
import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { DraftRestoreDialog, MultipleDraftsDialog } from './draft-restore-dialog'

// Mock setTimeout/other browser APIs that might be needed
vi.useFakeTimers()

describe('DraftRestoreDialog', () => {
  const mockDraft = {
    data: { name: 'Test App', description: 'Test Description' },
    timestamp: Date.now() - 3600000, // 1 hour ago
    version: '1.0',
  }

  const mockHandlers = {
    onRestore: vi.fn(),
    onDiscard: vi.fn(),
    onClose: vi.fn(),
  }

  it('should call onRestore when restore button is clicked', async () => {
    const user = userEvent.setup()
    render(<DraftRestoreDialog open={true} draft={mockDraft} {...mockHandlers} />)

    const restoreButton = screen.getByTestId('draft-restore-button')
    await user.click(restoreButton)

    expect(mockHandlers.onRestore).toHaveBeenCalledTimes(1)
  })

  it('should call onDiscard when discard button is clicked', async () => {
    const user = userEvent.setup()
    render(<DraftRestoreDialog open={true} draft={mockDraft} {...mockHandlers} />)

    const discardButton = screen.getByTestId('draft-discard-button')
    await user.click(discardButton)

    expect(mockHandlers.onDiscard).toHaveBeenCalledTimes(1)
  })
})

describe('MultipleDraftsDialog', () => {
  const mockDrafts = [
    { draftKey: 'client-app-draft-realm1-create-new', timestamp: Date.now(), version: '1.0' },
    {
      draftKey: 'client-app-draft-realm1-edit-123',
      timestamp: Date.now() - 3600000,
      version: '1.0',
    },
    {
      draftKey: 'client-app-draft-realm1-edit-456',
      timestamp: Date.now() - 7200000,
      version: '1.0',
    },
  ]

  const mockHandlers = {
    onRestore: vi.fn(),
    onDiscardAll: vi.fn(),
    onClose: vi.fn(),
  }

  it('should render dialog with all drafts', () => {
    render(<MultipleDraftsDialog open={true} drafts={mockDrafts} {...mockHandlers} />)

    expect(screen.getByTestId('multiple-drafts-dialog')).toBeInTheDocument()

    // Check if all drafts are displayed
    expect(screen.getByText('client-app-draft-realm1-create-new')).toBeInTheDocument()
    expect(screen.getByText('client-app-draft-realm1-edit-123')).toBeInTheDocument()
    expect(screen.getByText('client-app-draft-realm1-edit-456')).toBeInTheDocument()
  })

  it('should call onRestore with correct draft key when restore is clicked', async () => {
    const user = userEvent.setup()
    render(<MultipleDraftsDialog open={true} drafts={mockDrafts} {...mockHandlers} />)

    const restoreButton = screen.getByTestId('restore-draft-client-app-draft-realm1-edit-123')
    await user.click(restoreButton)

    expect(mockHandlers.onRestore).toHaveBeenCalledTimes(1)
    expect(mockHandlers.onRestore).toHaveBeenCalledWith('client-app-draft-realm1-edit-123')
  })

  it('should call onDiscardAll when discard all button is clicked', async () => {
    const user = userEvent.setup()
    render(<MultipleDraftsDialog open={true} drafts={mockDrafts} {...mockHandlers} />)

    const discardAllButton = screen.getByTestId('discard-all-drafts-button')
    await user.click(discardAllButton)

    expect(mockHandlers.onDiscardAll).toHaveBeenCalledTimes(1)
  })

  it('should call onClose when cancel is clicked', async () => {
    const user = userEvent.setup()
    render(<MultipleDraftsDialog open={true} drafts={mockDrafts} {...mockHandlers} />)

    const cancelButton = screen.getByTestId('multiple-drafts-cancel-button')
    await user.click(cancelButton)

    expect(mockHandlers.onClose).toHaveBeenCalledTimes(1)
  })

  afterEach(() => {
    vi.useRealTimers()
  })
})
