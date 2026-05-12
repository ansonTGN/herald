import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import { Spinner } from './spinner'

describe('Spinner component', () => {
  it('should render with default medium size', () => {
    render(<Spinner data-testid="test-spinner" />)

    const spinner = screen.getByTestId('test-spinner')
    expect(spinner).toBeInTheDocument()
    expect(spinner).toHaveClass('w-4', 'h-4')
  })

  it('should render with small size', () => {
    render(<Spinner data-testid="test-spinner" size="sm" />)

    const spinner = screen.getByTestId('test-spinner')
    expect(spinner).toHaveClass('w-3', 'h-3')
  })

  it('should render with large size', () => {
    render(<Spinner data-testid="test-spinner" size="lg" />)

    const spinner = screen.getByTestId('test-spinner')
    expect(spinner).toHaveClass('w-6', 'h-6')
  })

  it('should have animation classes', () => {
    render(<Spinner data-testid="test-spinner" />)

    const spinner = screen.getByTestId('test-spinner')
    expect(spinner).toHaveClass('animate-spin')
  })

  it('should have accessibility attributes', () => {
    render(<Spinner data-testid="test-spinner" />)

    const spinner = screen.getByTestId('test-spinner')
    expect(spinner).toHaveAttribute('role', 'status')
    expect(spinner).toHaveAttribute('aria-label', 'Loading')
  })

  it('should have screen reader text', () => {
    render(<Spinner data-testid="test-spinner" />)

    const spinner = screen.getByTestId('test-spinner')
    const srText = spinner.querySelector('.sr-only')
    expect(srText).toBeInTheDocument()
    expect(srText).toHaveTextContent('Loading...')
  })
})
