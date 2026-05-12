import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useAppForm } from '@/components/ui/tanstack-form'
import { TextareaField } from './textarea-field'

describe('TextareaField component', () => {
  it('should render with data-testid attribute', () => {
    const form = useAppForm({
      defaultValues: {
        testDescription: '',
      },
    })

    render(
      <TextareaField
        form={form}
        name="testDescription"
        label="Test Description"
        dataTestId="test-description-input"
      />
    )

    const textarea = screen.queryByTestId('test-description-input')
    expect(textarea).toBeInTheDocument()
    expect(textarea?.getAttribute('data-testid')).toBe('test-description-input')
  })

  it('should call onSubmit when Enter key is pressed', async () => {
    const user = userEvent.setup()
    const handleSubmit = vi.fn()

    const form = useAppForm({
      defaultValues: {
        testDescription: '',
      },
    })

    render(
      <TextareaField
        form={form}
        name="testDescription"
        label="Test Description"
        dataTestId="test-description-input"
        onSubmit={handleSubmit}
      />
    )

    const textarea = screen.getByTestId('test-description-input')

    await user.type(textarea, 'Test Description')

    // Press Enter key
    await user.keyboard('{Enter}')

    expect(handleSubmit).toHaveBeenCalledTimes(1)
  })

  it('should not call onSubmit when Enter key is pressed if onSubmit is not provided', async () => {
    const user = userEvent.setup()
    const handleSubmit = vi.fn()

    const form = useAppForm({
      defaultValues: {
        testDescription: '',
      },
    })

    render(
      <TextareaField
        form={form}
        name="testDescription"
        label="Test Description"
        dataTestId="test-description-input"
      />
    )

    const textarea = screen.getByTestId('test-description-input')

    await user.type(textarea, 'Test Description')

    // Press Enter key
    await user.keyboard('{Enter}')

    expect(handleSubmit).not.toHaveBeenCalled()
  })

  it('should prevent default form submission when Enter is pressed', async () => {
    const user = userEvent.setup()
    const handleSubmit = vi.fn()

    const form = useAppForm({
      defaultValues: {
        testDescription: '',
      },
    })

    render(
      <TextareaField
        form={form}
        name="testDescription"
        label="Test Description"
        dataTestId="test-description-input"
        onSubmit={handleSubmit}
      />
    )

    const textarea = screen.getByTestId('test-description-input')

    await user.type(textarea, 'Test Description')
    await user.keyboard('{Enter}')

    expect(handleSubmit).toHaveBeenCalledTimes(1)
  })
})
