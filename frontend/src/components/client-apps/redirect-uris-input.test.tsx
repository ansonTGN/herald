import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RedirectUrisInput, type UriItem } from './redirect-uris-input'

describe('RedirectUrisInput', () => {
  const mockOnChange = vi.fn()

  beforeEach(() => {
    mockOnChange.mockClear()
  })

  it('should add URI when Add button is clicked', async () => {
    const user = userEvent.setup()
    render(<RedirectUrisInput value={[]} onChange={mockOnChange} dataTestId="test-uris" />)

    const input = screen.getByTestId('test-uris-field')
    const addButton = screen.getByTestId('test-uris-add-button')

    await user.type(input, 'https://example.com/callback')
    await user.click(addButton)

    expect(mockOnChange).toHaveBeenCalledWith([
      { id: expect.any(String), value: 'https://example.com/callback', isValid: true },
    ])
  })

  it('should add URI when Enter key is pressed', async () => {
    const user = userEvent.setup()
    render(<RedirectUrisInput value={[]} onChange={mockOnChange} dataTestId="test-uris" />)

    const input = screen.getByTestId('test-uris-field')

    await user.type(input, 'https://example.com/callback{Enter}')

    expect(mockOnChange).toHaveBeenCalledWith([
      { id: expect.any(String), value: 'https://example.com/callback', isValid: true },
    ])
  })

  it('should not add invalid URI', async () => {
    const user = userEvent.setup()
    render(<RedirectUrisInput value={[]} onChange={mockOnChange} dataTestId="test-uris" />)

    const input = screen.getByTestId('test-uris-field')
    const addButton = screen.getByTestId('test-uris-add-button')

    await user.type(input, 'not-a-valid-uri')
    await user.click(addButton)

    expect(mockOnChange).not.toHaveBeenCalled()
    expect(screen.getByText(/Must be a valid URL/)).toBeInTheDocument()
  })

  it('should remove URI when remove button is clicked', async () => {
    const user = userEvent.setup()
    const items: UriItem[] = [
      { id: '1', value: 'https://example.com/callback', isValid: true },
      { id: '2', value: 'https://app.example.com/auth', isValid: true },
    ]

    render(<RedirectUrisInput value={items} onChange={mockOnChange} dataTestId="test-uris" />)

    const removeButton = screen.getByTestId('remove-uri-1')
    await user.click(removeButton)

    expect(mockOnChange).toHaveBeenCalledWith([
      { id: '2', value: 'https://app.example.com/auth', isValid: true },
    ])
  })

  it('should prevent duplicate URIs', async () => {
    const user = userEvent.setup()
    const items: UriItem[] = [{ id: '1', value: 'https://example.com/callback', isValid: true }]

    render(<RedirectUrisInput value={items} onChange={mockOnChange} dataTestId="test-uris" />)

    const input = screen.getByTestId('test-uris-field')
    const addButton = screen.getByTestId('test-uris-add-button')

    await user.type(input, 'https://example.com/callback')
    await user.click(addButton)

    expect(mockOnChange).not.toHaveBeenCalled()
    expect(screen.getByText(/This URI is already in the list/)).toBeInTheDocument()
  })

  it('should call onSubmit when Enter is pressed on empty input', async () => {
    const user = userEvent.setup()
    const mockOnSubmit = vi.fn()
    render(
      <RedirectUrisInput
        value={[]}
        onChange={mockOnChange}
        onSubmit={mockOnSubmit}
        dataTestId="test-uris"
      />
    )

    const input = screen.getByTestId('test-uris-field')
    await user.type(input, '{Enter}')

    expect(mockOnChange).not.toHaveBeenCalled()
    expect(mockOnSubmit).toHaveBeenCalledWith()
  })

  it('should add URI when Enter is pressed with valid input', async () => {
    const user = userEvent.setup()
    const mockOnSubmit = vi.fn()
    render(
      <RedirectUrisInput
        value={[]}
        onChange={mockOnChange}
        onSubmit={mockOnSubmit}
        dataTestId="test-uris"
      />
    )

    const input = screen.getByTestId('test-uris-field')
    await user.type(input, 'https://example.com/callback{Enter}')

    expect(mockOnChange).toHaveBeenCalledWith([
      { id: expect.any(String), value: 'https://example.com/callback', isValid: true },
    ])
    expect(mockOnSubmit).not.toHaveBeenCalled()
  })
})
