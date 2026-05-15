import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Spinner } from './spinner'

describe('Spinner component', () => {
  it('should have accessibility attributes', () => {
    render(<Spinner data-testid="test-spinner" />)

    const spinner = screen.getByTestId('test-spinner')
    expect(spinner).toHaveAttribute('role', 'status')
    expect(spinner).toHaveAttribute('aria-label', 'Loading')
  })
})
