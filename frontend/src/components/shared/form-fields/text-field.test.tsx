import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useAppForm } from '@/components/ui/tanstack-form'
import { TextField } from './text-field'

describe('TextField component', () => {
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
    await user.keyboard('{Enter}')

    expect(handleSubmit).toHaveBeenCalledTimes(1)
  })
})
