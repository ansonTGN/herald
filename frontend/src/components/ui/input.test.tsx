import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Input } from './input'

describe('Input component data-testid', () => {
  it('should render with data-testid attribute', () => {
    render(<Input data-testid="test-input" />)

    const input = screen.queryByTestId('test-input')
    expect(input).toBeInTheDocument()
    expect(input?.getAttribute('data-testid')).toBe('test-input')
  })

  it('should render with data-testid and other props', () => {
    render(<Input data-testid="email-input" type="email" placeholder="test@example.com" />)

    const input = screen.getByTestId('email-input')
    expect(input.getAttribute('type')).toBe('email')
    expect(input.getAttribute('placeholder')).toBe('test@example.com')
  })
})
