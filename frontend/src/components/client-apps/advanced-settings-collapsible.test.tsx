import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AdvancedSettingsCollapsible } from './advanced-settings-collapsible'

describe('AdvancedSettingsCollapsible', () => {
  it('renders collapsible component', () => {
    render(
      <AdvancedSettingsCollapsible>
        <div>Test content</div>
      </AdvancedSettingsCollapsible>
    )

    expect(screen.getByTestId('advanced-settings-collapsible')).toBeInTheDocument()
    expect(screen.getByTestId('advanced-settings-collapsible-trigger')).toBeInTheDocument()
    expect(screen.getByText('Advanced Security Settings')).toBeInTheDocument()
    expect(screen.getByText('8 options')).toBeInTheDocument()
  })

  it('renders with custom test id', () => {
    render(
      <AdvancedSettingsCollapsible dataTestId="custom-test-id">
        <div>Test content</div>
      </AdvancedSettingsCollapsible>
    )

    expect(screen.getByTestId('custom-test-id')).toBeInTheDocument()
    expect(screen.getByTestId('custom-test-id-trigger')).toBeInTheDocument()
  })

  it('is collapsed by default', () => {
    render(
      <AdvancedSettingsCollapsible>
        <div data-testid="test-content">Test content</div>
      </AdvancedSettingsCollapsible>
    )

    expect(screen.queryByTestId('advanced-settings-collapsible-content')).not.toBeInTheDocument()
    expect(screen.queryByTestId('test-content')).not.toBeInTheDocument()
  })

  it('expands when trigger is clicked', async () => {
    const user = userEvent.setup()
    render(
      <AdvancedSettingsCollapsible>
        <div data-testid="test-content">Test content</div>
      </AdvancedSettingsCollapsible>
    )

    const trigger = screen.getByTestId('advanced-settings-collapsible-trigger')
    await user.click(trigger)

    expect(screen.getByTestId('advanced-settings-collapsible-content')).toBeInTheDocument()
    expect(screen.getByTestId('test-content')).toBeInTheDocument()
  })

  it('collapses when trigger is clicked again', async () => {
    const user = userEvent.setup()
    render(
      <AdvancedSettingsCollapsible>
        <div data-testid="test-content">Test content</div>
      </AdvancedSettingsCollapsible>
    )

    const trigger = screen.getByTestId('advanced-settings-collapsible-trigger')

    // Expand
    await user.click(trigger)
    expect(screen.getByTestId('advanced-settings-collapsible-content')).toBeInTheDocument()

    // Collapse
    await user.click(trigger)
    expect(screen.queryByTestId('advanced-settings-collapsible-content')).not.toBeInTheDocument()
  })

  it('displays description text correctly', () => {
    render(
      <AdvancedSettingsCollapsible>
        <div>Test content</div>
      </AdvancedSettingsCollapsible>
    )

    expect(
      screen.getByText('Additional security options for advanced use cases')
    ).toBeInTheDocument()
  })

  it('renders children when expanded', async () => {
    const user = userEvent.setup()
    render(
      <AdvancedSettingsCollapsible>
        <div data-testid="child-1">Child 1</div>
        <div data-testid="child-2">Child 2</div>
      </AdvancedSettingsCollapsible>
    )

    const trigger = screen.getByTestId('advanced-settings-collapsible-trigger')
    await user.click(trigger)

    expect(screen.getByTestId('child-1')).toBeInTheDocument()
    expect(screen.getByTestId('child-2')).toBeInTheDocument()
  })
})
