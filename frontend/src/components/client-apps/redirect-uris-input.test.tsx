import { describe, it, expect, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RedirectUrisInput, type UriItem } from './redirect-uris-input'

describe('RedirectUrisInput', () => {
  const mockOnChange = vi.fn()

  beforeEach(() => {
    mockOnChange.mockClear()
  })

  it('should render input field with placeholder', () => {
    render(
      <RedirectUrisInput
        value={[]}
        onChange={mockOnChange}
        placeholder="https://example.com/callback"
      />
    )

    expect(screen.getByPlaceholderText('https://example.com/callback')).toBeInTheDocument()
  })

  it('should render label when provided', () => {
    render(<RedirectUrisInput value={[]} onChange={mockOnChange} label="Test Label" />)

    expect(screen.getByText('Test Label')).toBeInTheDocument()
  })

  it('should render required indicator when required is true', () => {
    render(<RedirectUrisInput value={[]} onChange={mockOnChange} label="Required Field" required />)

    expect(screen.getByText('Required Field')).toBeInTheDocument()
    expect(screen.getByText('*')).toBeInTheDocument()
  })

  it('should render existing URI items', () => {
    const items: UriItem[] = [
      { id: '1', value: 'https://example.com/callback', isValid: true },
      { id: '2', value: 'https://app.example.com/auth', isValid: true },
    ]

    render(<RedirectUrisInput value={items} onChange={mockOnChange} />)

    expect(screen.getByText('https://example.com/callback')).toBeInTheDocument()
    expect(screen.getByText('https://app.example.com/auth')).toBeInTheDocument()
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

  it('should show green checkmark for valid URIs', async () => {
    const user = userEvent.setup()
    render(<RedirectUrisInput value={[]} onChange={mockOnChange} dataTestId="test-uris" />)

    const input = screen.getByTestId('test-uris-field')
    await user.type(input, 'https://example.com/callback')

    // Check for the checkmark icon (should be in the document when input is valid)
    const checkmark = screen.getByRole('img', { hidden: true })?.querySelector('svg')
    expect(checkmark).toBeTruthy()
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

  it('should render help text when provided', () => {
    render(
      <RedirectUrisInput
        value={[]}
        onChange={mockOnChange}
        helpText="Enter one URI per line"
        dataTestId="test-uris"
      />
    )

    expect(screen.getByText('Enter one URI per line')).toBeInTheDocument()
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
