import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useAppForm } from '@/components/ui/tanstack-form'
import { TextareaField } from './textarea-field'

describe('TextareaField component', () => {
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
    await user.keyboard('{Enter}')

    expect(handleSubmit).toHaveBeenCalledTimes(1)
  })
})
