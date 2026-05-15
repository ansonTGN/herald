import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { PermissionCheckboxList } from '../permission-checkbox-list'
import type { PermissionResponse } from '@/lib/api-generated'

describe('PermissionCheckboxList', () => {
  const mockPermissions: PermissionResponse[] = [
    {
      id: '1',
      name: 'users.view',
      resource: 'users',
      action: 'view',
      description: 'View users',
      realmId: 'realm-1',
      isBuiltin: false,
    },
    {
      id: '2',
      name: 'users.manage',
      resource: 'users',
      action: 'manage',
      description: 'Manage users',
      realmId: 'realm-1',
      isBuiltin: true,
    },
    {
      id: '3',
      name: 'roles.view',
      resource: 'roles',
      action: 'view',
      description: 'View roles',
      realmId: 'realm-1',
      isBuiltin: false,
    },
  ]

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN permissions array is provided WHEN rendering THEN should group by resource', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <PermissionCheckboxList
        permissions={mockPermissions}
        assignedPermissionIds={[]}
        onTogglePermission={handleChange}
        isBuiltinRole={false}
      />
    )

    expect(screen.getByTestId('permission-checkbox-list')).toBeInTheDocument()

    // Check for resource badges - use exact match to avoid matching permission names like 'users.view'
    expect(screen.getByText('users', { exact: true })).toBeInTheDocument()
    expect(screen.getByText('roles', { exact: true })).toBeInTheDocument()
  })

  it('GIVEN permission checkbox list is rendered WHEN user selects permission THEN should call onChange with ID', async () => {
    const handleChange = vi.fn()
    render(
      <PermissionCheckboxList
        permissions={mockPermissions}
        assignedPermissionIds={[]}
        onTogglePermission={handleChange}
        isBuiltinRole={false}
      />
    )

    // Find and click the actual checkbox input element
    const checkboxInput = screen.getByLabelText('users.view')
    await userEvent.click(checkboxInput)

    expect(handleChange).toHaveBeenCalledTimes(1)
    expect(handleChange).toHaveBeenCalledWith('1', true)
  })

  it('GIVEN permission is already assigned WHEN user deselects THEN should call onChange with false', async () => {
    const handleChange = vi.fn()
    render(
      <PermissionCheckboxList
        permissions={mockPermissions}
        assignedPermissionIds={['1']}
        onTogglePermission={handleChange}
        isBuiltinRole={false}
      />
    )

    // Find and click the actual checkbox input element
    const checkboxInput = screen.getByLabelText('users.view')
    await userEvent.click(checkboxInput)

    expect(handleChange).toHaveBeenCalledTimes(1)
    expect(handleChange).toHaveBeenCalledWith('1', false)
  })

  it('GIVEN permission is builtin in builtin role WHEN rendering THEN should disable checkbox', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <PermissionCheckboxList
        permissions={mockPermissions}
        assignedPermissionIds={['2']}
        onTogglePermission={handleChange}
        isBuiltinRole={true}
      />
    )

    const builtinCheckbox = screen.getByTestId('permission-checkbox-2')
    expect(builtinCheckbox).toBeDisabled()
  })

  it('GIVEN permission is builtin in custom role WHEN rendering THEN should enable checkbox', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <PermissionCheckboxList
        permissions={mockPermissions}
        assignedPermissionIds={[]}
        onTogglePermission={handleChange}
        isBuiltinRole={false}
      />
    )

    const builtinCheckbox = screen.getByTestId('permission-checkbox-2')
    expect(builtinCheckbox).toBeEnabled()
  })

  it('GIVEN disabled prop is true WHEN rendering THEN should disable all checkboxes', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <PermissionCheckboxList
        permissions={mockPermissions}
        assignedPermissionIds={[]}
        onTogglePermission={handleChange}
        isBuiltinRole={false}
        disabled={true}
      />
    )

    const checkbox1 = screen.getByTestId('permission-checkbox-1')
    const checkbox2 = screen.getByTestId('permission-checkbox-2')
    const checkbox3 = screen.getByTestId('permission-checkbox-3')

    expect(checkbox1).toBeDisabled()
    expect(checkbox2).toBeDisabled()
    expect(checkbox3).toBeDisabled()
  })

  it('GIVEN permissions array is empty WHEN rendering THEN should show empty state', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <PermissionCheckboxList
        permissions={[]}
        assignedPermissionIds={[]}
        onTogglePermission={handleChange}
        isBuiltinRole={false}
      />
    )

    expect(screen.getByText('No permissions available')).toBeInTheDocument()
  })

  it('GIVEN role is builtin WHEN rendering THEN should show warning message', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <PermissionCheckboxList
        permissions={mockPermissions}
        assignedPermissionIds={[]}
        onTogglePermission={handleChange}
        isBuiltinRole={true}
      />
    )

    expect(screen.getByText(/Built-in permissions cannot be removed/)).toBeInTheDocument()
  })

  it('GIVEN permissions are grouped WHEN rendering THEN should display count for each resource', async () => {
    const handleChange = vi.fn()
    const screen = render(
      <PermissionCheckboxList
        permissions={mockPermissions}
        assignedPermissionIds={[]}
        onTogglePermission={handleChange}
        isBuiltinRole={false}
      />
    )

    // users resource has 2 permissions
    expect(screen.getByText('(2 permissions)')).toBeInTheDocument()

    // roles resource has 1 permission
    expect(screen.getByText('(1 permission)')).toBeInTheDocument()
  })
})
