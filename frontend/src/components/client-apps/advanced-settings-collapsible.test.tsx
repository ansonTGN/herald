import { describe, it, expect } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { AdvancedSettingsCollapsible } from './advanced-settings-collapsible'

describe('AdvancedSettingsCollapsible', () => {
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
})
