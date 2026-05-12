import { describe, it, expect } from 'vitest'
import { render } from '@testing-library/react'
import { Route } from '../login'

describe('LoginPage data-testid attributes', () => {
  it('defines a login route component', () => {
    expect(Route).toBeDefined()
    expect(Route.options.component).toBeDefined()
  })

  it('should render Input component with data-testid prop', () => {
    const { container } = render(<input data-testid="test-input" type="text" />)

    const input = container.querySelector('[data-testid="test-input"]')
    expect(input).toBeInTheDocument()
    expect(input?.getAttribute('data-testid')).toBe('test-input')
  })
})
