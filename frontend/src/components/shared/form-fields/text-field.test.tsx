import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useAppForm } from '@/components/ui/tanstack-form'
import { TextField } from './text-field'

describe('TextField component', () => {
  it('should render with data-testid attribute', () => {
    const form = useAppForm({
      defaultValues: {
        testName: '',
      },
    })

    render(<TextField form={form} name="testName" label="Test Name" dataTestId="test-name-input" />)

    const input = screen.queryByTestId('test-name-input')
    expect(input).toBeInTheDocument()
    expect(input?.getAttribute('data-testid')).toBe('test-name-input')
  })

  it('should call onSubmit when Enter key is pressed', async () => {
    const user = userEvent.setup()
    const handleSubmit = vi.fn()

    const form = useAppForm({
      defaultValues: {
        testName: '',
      },
    })

    render(
      <TextField
        form={form}
        name="testName"
        label="Test Name"
        dataTestId="test-name-input"
        onSubmit={handleSubmit}
      />
    )

    const input = screen.getByTestId('test-name-input')

    await user.type(input, 'Test Value')

    // Press Enter key
    await user.keyboard('{Enter}')

    expect(handleSubmit).toHaveBeenCalledTimes(1)
  })

  it('should not call onSubmit when Enter key is pressed if onSubmit is not provided', async () => {
    const user = userEvent.setup()
    const handleSubmit = vi.fn()

    const form = useAppForm({
      defaultValues: {
        testName: '',
      },
    })

    render(<TextField form={form} name="testName" label="Test Name" dataTestId="test-name-input" />)

    const input = screen.getByTestId('test-name-input')

    await user.type(input, 'Test Value')

    // Press Enter key
    await user.keyboard('{Enter}')

    expect(handleSubmit).not.toHaveBeenCalled()
  })

  it('should prevent default form submission when Enter is pressed', async () => {
    const user = userEvent.setup()
    const handleSubmit = vi.fn()

    const form = useAppForm({
      defaultValues: {
        testName: '',
      },
    })

    render(
      <TextField
        form={form}
        name="testName"
        label="Test Name"
        dataTestId="test-name-input"
        onSubmit={handleSubmit}
      />
    )

    const input = screen.getByTestId('test-name-input')

    await user.type(input, 'Test Value')
    await user.keyboard('{Enter}')

    expect(handleSubmit).toHaveBeenCalledTimes(1)
  })
})
