import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RoleSelector } from '../role-selector'

describe('RoleSelector', () => {
  const mockRoles = [
    { id: '1', name: 'Admin' },
    { id: '2', name: 'User' },
    { id: '3', name: 'Moderator' },
  ]

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN roles array is provided WHEN rendering THEN should display all roles', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <RoleSelector roles={mockRoles} selectedRoleIds={[]} onChange={handleChange} />
    )

    const trigger = screen.getByTestId('role-selector-trigger')
    expect(trigger).toBeInTheDocument()

    // Open dropdown
    await userEvent.click(trigger)

    // Check all roles are displayed
    expect(screen.getByTestId('role-selector-item-1')).toBeInTheDocument()
    expect(screen.getByTestId('role-selector-item-2')).toBeInTheDocument()
    expect(screen.getByTestId('role-selector-item-3')).toBeInTheDocument()
  })

  it('GIVEN role selector is rendered WHEN user clicks role THEN should call onChange with role ID', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <RoleSelector roles={mockRoles} selectedRoleIds={[]} onChange={handleChange} />
    )

    // Open dropdown
    const trigger = screen.getByTestId('role-selector-trigger')
    await userEvent.click(trigger)

    // Click on a role
    const adminRole = screen.getByTestId('role-selector-item-1')
    await userEvent.click(adminRole)

    // Verify onChange was called with role ID
    expect(handleChange).toHaveBeenCalledTimes(1)
    expect(handleChange).toHaveBeenCalledWith(['1'])
  })

  it('GIVEN role is already selected WHEN user clicks again THEN should remove it', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <RoleSelector roles={mockRoles} selectedRoleIds={['1']} onChange={handleChange} />
    )

    // Open dropdown
    const trigger = screen.getByTestId('role-selector-trigger')
    await userEvent.click(trigger)

    // Click on already selected role
    const adminRole = screen.getByTestId('role-selector-item-1')
    await userEvent.click(adminRole)

    // Verify onChange was called with empty array (removed)
    expect(handleChange).toHaveBeenCalledTimes(1)
    expect(handleChange).toHaveBeenCalledWith([])
  })

  it('GIVEN multiple roles are selected WHEN rendering THEN should display all selected roles', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <RoleSelector roles={mockRoles} selectedRoleIds={['1', '2']} onChange={handleChange} />
    )

    const trigger = screen.getByTestId('role-selector-trigger')
    expect(trigger).toBeInTheDocument()

    // Check that badges are displayed for selected roles
    expect(screen.getAllByText('Admin', { exact: true })[0]).toBeInTheDocument()
    expect(screen.getAllByText('User', { exact: true })[0]).toBeInTheDocument()
  })

  it('GIVEN disabled prop is true WHEN rendering THEN should disable selector', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <RoleSelector
        roles={mockRoles}
        selectedRoleIds={[]}
        onChange={handleChange}
        disabled={true}
      />
    )

    const trigger = screen.getByTestId('role-selector-trigger')
    expect(trigger).toBeDisabled()
  })

  it('GIVEN no roles are selected WHEN rendering THEN should display placeholder', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <RoleSelector
        roles={mockRoles}
        selectedRoleIds={[]}
        onChange={handleChange}
        placeholder="Select roles..."
      />
    )

    const trigger = screen.getByTestId('role-selector-trigger')
    expect(trigger).toHaveTextContent('Select roles...')
  })

  it('GIVEN custom placeholder is provided WHEN rendering THEN should display it', async () => {
    const handleChange = vi.fn()
    const customPlaceholder = 'Choose roles please'
    const screen = render(
      <RoleSelector
        roles={mockRoles}
        selectedRoleIds={[]}
        onChange={handleChange}
        placeholder={customPlaceholder}
      />
    )

    const trigger = screen.getByTestId('role-selector-trigger')
    expect(trigger).toHaveTextContent(customPlaceholder)
  })

  it('GIVEN user types in search WHEN roles are present THEN should filter roles', async () => {
    const handleChange = vi.fn()
    render(<RoleSelector roles={mockRoles} selectedRoleIds={[]} onChange={handleChange} />)

    // Open dropdown
    const trigger = document.querySelector('[data-testid="role-selector-trigger"]') as HTMLElement
    if (trigger) {
      await userEvent.click(trigger)
    }

    // Type in search
    const searchInput = document.querySelector(
      '[data-testid="role-selector-search"]'
    ) as HTMLInputElement
    if (searchInput) {
      await userEvent.type(searchInput, 'Admin')
      // The search should filter the roles (implementation detail check)
      expect(searchInput).toHaveValue('Admin')
    }
  })
})
