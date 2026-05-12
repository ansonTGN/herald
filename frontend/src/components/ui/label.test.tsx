import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Label } from './label'

describe('Label component data-testid', () => {
  it('should render with data-testid attribute', () => {
    render(<Label data-testid="test-label">Test Label</Label>)

    const label = screen.queryByTestId('test-label')
    expect(label).toBeInTheDocument()
    expect(label?.getAttribute('data-testid')).toBe('test-label')
  })

  it('should render with htmlFor and data-testid', () => {
    render(
      <Label data-testid="email-label" htmlFor="email">
        Email
      </Label>
    )

    const label = screen.getByTestId('email-label')
    expect(label.getAttribute('for')).toBe('email')
  })
})
