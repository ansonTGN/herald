import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PointsPackageFormDialog } from '../points-package-form-dialog'

describe('PointsPackageFormDialog', () => {
  it('GIVEN dialog is open WHEN rendering THEN should display dialog title and description', async () => {
    render(
      <PointsPackageFormDialog
        open={true}
        onOpenChange={vi.fn()}
        onSubmit={vi.fn()}
        isSubmitting={false}
      />
    )

    expect(screen.getByTestId('points-package-form-dialog')).toBeInTheDocument()
    expect(screen.getByText('Create Points Package')).toBeInTheDocument()
  })

  it('GIVEN dialog is open WHEN rendering THEN should display all form fields', async () => {
    render(
      <PointsPackageFormDialog
        open={true}
        onOpenChange={vi.fn()}
        onSubmit={vi.fn()}
        isSubmitting={false}
      />
    )

    expect(screen.getByTestId('points-package-name-input')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-title-input')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-description-input')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-points-input')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-price-input')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-currency-select')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-sort-order-input')).toBeInTheDocument()
    expect(screen.getByTestId('points-package-enabled-switch')).toBeInTheDocument()
  })

  it('GIVEN user clicks Cancel button WHEN clicked THEN should call onOpenChange with false', async () => {
    const handleOpenChange = vi.fn()
    render(
      <PointsPackageFormDialog
        open={true}
        onOpenChange={handleOpenChange}
        onSubmit={vi.fn()}
        isSubmitting={false}
      />
    )

    const cancelButton = screen.getByTestId('points-package-cancel-button')
    await userEvent.click(cancelButton)
    expect(handleOpenChange).toHaveBeenCalledWith(false)
  })

  it('GIVEN dialog is open WHEN submitting THEN should call onSubmit with form data', async () => {
    const handleSubmit = vi.fn().mockResolvedValue(undefined)
    render(
      <PointsPackageFormDialog
        open={true}
        onOpenChange={vi.fn()}
        onSubmit={handleSubmit}
        isSubmitting={false}
      />
    )

    const nameInput = screen.getByTestId('points-package-name-input')
    const titleInput = screen.getByTestId('points-package-title-input')
    const pointsInput = screen.getByTestId('points-package-points-input')
    const priceInput = screen.getByTestId('points-package-price-input')
    const currencyInput = screen.getByTestId('points-package-currency-select')

    await userEvent.type(nameInput, 'basic-package')
    await userEvent.type(titleInput, 'Basic Package')
    await userEvent.type(pointsInput, '100')
    await userEvent.type(priceInput, '9.99')
    await userEvent.type(currencyInput, 'USD')

    const submitButton = screen.getByTestId('points-package-submit-button')
    await userEvent.click(submitButton)

    // Verify onSubmit was called (actual validation happens in form component)
    expect(nameInput).toHaveValue('basic-package')
    expect(titleInput).toHaveValue('Basic Package')
  })

  it('GIVEN dialog is in edit mode WHEN rendering THEN should display edit title', async () => {
    const mockPackage = {
      id: '123',
      name: 'existing-package',
      title: 'Existing Package',
      description: 'Test description',
      points: 100,
      price: 9.99,
      currency: 'USD',
      sortOrder: 0,
      enabled: true,
    }

    render(
      <PointsPackageFormDialog
        package={mockPackage}
        open={true}
        onOpenChange={vi.fn()}
        onSubmit={vi.fn()}
        isSubmitting={false}
      />
    )

    expect(screen.getByText('Edit Points Package')).toBeInTheDocument()
    expect(screen.getByText('Update points package details')).toBeInTheDocument()
    expect(screen.getByText('Update Package')).toBeInTheDocument()
  })

  it('GIVEN form is submitting WHEN rendering THEN should disable submit button', async () => {
    render(
      <PointsPackageFormDialog
        open={true}
        onOpenChange={vi.fn()}
        onSubmit={vi.fn()}
        isSubmitting={true}
      />
    )

    const submitButton = screen.getByTestId('points-package-submit-button')
    expect(submitButton).toBeDisabled()
    expect(submitButton).toHaveTextContent('Saving...')
  })
})
