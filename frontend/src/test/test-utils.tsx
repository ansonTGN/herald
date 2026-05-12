import { render } from 'vitest-browser-react'
import { userEvent } from '@testing-library/user-event'

/**
 * Test wrapper component with required context providers.
 * Currently minimal - expand as needed for tests.
 */
export function TestWrapper({ children }: { children: React.ReactNode }) {
  return <>{children}</>
}

/**
 * Custom render function with test wrapper.
 * Provides consistent test setup across all component tests.
 *
 * @example
 * const screen = await render(<MyComponent />, { wrapper: TestWrapper })
 */
export const customRender = async (ui: React.ReactNode) => {
  return render(ui)
}

/**
 * Setup user event with consistent configuration.
 * Uses `{ delay: null }` to avoid unnecessary delays in tests.
 *
 * @example
 * const user = setupUserEvent()
 */
export function setupUserEvent() {
  return userEvent.setup({ delay: null })
}

/**
 * Test timeout constants for consistent test configuration.
 */
export const TEST_TIMEOUTS = {
  DEFAULT: 5000,
  SLOW_TEST: 10000,
  VERY_SLOW_TEST: 15000,
} as const
