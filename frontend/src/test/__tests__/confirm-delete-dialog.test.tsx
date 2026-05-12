import { describe, expect, it, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { ConfirmDeleteDialog } from '@/components/shared'

describe('ConfirmDeleteDialog', () => {
  it('renders the shared destructive dialog shell with stable test ids', () => {
    render(
      <ConfirmDeleteDialog
        open={true}
        onOpenChange={vi.fn()}
        title="Delete Product"
        description="Delete this product permanently."
        onConfirm={vi.fn()}
        contentTestId="confirm-dialog"
        cancelTestId="confirm-dialog-cancel"
        confirmTestId="confirm-dialog-confirm"
      />
    )

    expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument()
    expect(screen.getByTestId('confirm-dialog-cancel')).toHaveTextContent('Cancel')
    expect(screen.getByTestId('confirm-dialog-confirm')).toHaveTextContent('Delete')
  })

  it('disables destructive actions while pending', async () => {
    const onConfirm = vi.fn()

    render(
      <ConfirmDeleteDialog
        open={true}
        onOpenChange={vi.fn()}
        title="Delete Product"
        description="Delete this product permanently."
        onConfirm={onConfirm}
        isPending={true}
        confirmTestId="confirm-dialog-confirm"
      />
    )

    const confirmButton = screen.getByTestId('confirm-dialog-confirm')
    expect(confirmButton).toBeDisabled()
    expect(confirmButton).toHaveTextContent('Delete...')

    await userEvent.click(confirmButton)
    expect(onConfirm).not.toHaveBeenCalled()
  })
})
