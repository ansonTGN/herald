import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { RolePermissionsDialog } from '../role-permissions-dialog'
import type { RoleResponse, PermissionResponse } from '@/lib/api-generated'

// Mock API and hooks
vi.mock('@/lib/api-generated', () => ({
  assignPermissionToRole: vi.fn().mockResolvedValue({
    data: { success: true },
  }),
  removePermissionFromRole: vi.fn().mockResolvedValue({
    data: { success: true },
  }),
}))

vi.mock('@/hooks/use-form-mutation', () => ({
  useFormMutation: () => ({
    isSubmitting: false,
    mutate: vi.fn().mockResolvedValue({}),
  }),
}))

describe('RolePermissionsDialog', () => {
  const mockRole: RoleResponse = {
    id: '1',
    name: 'realm-admin',
    description: 'Realm administrator',
    realmId: 'realm-1',
    clientId: 'client-1',
    isBuiltin: true,
  }

  const mockPermissions: PermissionResponse[] = [
    {
      id: '1',
      name: 'users.view',
      resource: 'users',
      action: 'view',
      description: 'View users',
      realmId: 'realm-1',
      isBuiltin: true,
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

  const assignedPermissionIds = ['1', '2']

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN dialog is open WHEN rendering THEN should display permission summary stats', async () => {
    const screen = render(
      <RolePermissionsDialog
        open={true}
        onOpenChange={vi.fn()}
        role={mockRole}
        realmId="realm-1"
        allPermissions={mockPermissions}
        assignedPermissionIds={assignedPermissionIds}
      />
    )

    expect(screen.getByText('2 / 3')).toBeInTheDocument()
    expect(screen.getByText('permissions assigned')).toBeInTheDocument()
  })

  it('GIVEN role is builtin WHEN rendering THEN should show warning message', async () => {
    const screen = render(
      <RolePermissionsDialog
        open={true}
        onOpenChange={vi.fn()}
        role={mockRole}
        realmId="realm-1"
        allPermissions={mockPermissions}
        assignedPermissionIds={assignedPermissionIds}
      />
    )

    // The warning appears in both DialogDescription and Alert
    expect(screen.getAllByText(/Built-in permissions cannot be removed/)[0]).toBeInTheDocument()
  })

  it('GIVEN role is not builtin WHEN rendering THEN should not show warning message', async () => {
    const customRole = { ...mockRole, isBuiltin: false }
    render(
      <RolePermissionsDialog
        open={true}
        onOpenChange={vi.fn()}
        role={customRole}
        realmId="realm-1"
        allPermissions={mockPermissions}
        assignedPermissionIds={assignedPermissionIds}
      />
    )

    expect(screen.queryByTestId('builtin-permission-warning')).not.toBeInTheDocument()
  })

  it('GIVEN user clicks Close button WHEN clicked THEN should call onOpenChange with false', async () => {
    const handleOpenChange = vi.fn()
    render(
      <RolePermissionsDialog
        open={true}
        onOpenChange={handleOpenChange}
        role={mockRole}
        realmId="realm-1"
        allPermissions={mockPermissions}
        assignedPermissionIds={assignedPermissionIds}
      />
    )

    const closeButton = screen.getByTestId('role-permissions-close-button')
    await userEvent.click(closeButton)

    expect(handleOpenChange).toHaveBeenCalledTimes(1)
    expect(handleOpenChange).toHaveBeenCalledWith(false)
  })

  it('GIVEN permissions are provided WHEN rendering THEN should group by resource', async () => {
    const screen = render(
      <RolePermissionsDialog
        open={true}
        onOpenChange={vi.fn()}
        role={mockRole}
        realmId="realm-1"
        allPermissions={mockPermissions}
        assignedPermissionIds={assignedPermissionIds}
      />
    )

    // Check for resource badges - they should appear multiple times (in badge and in the count text)
    expect(screen.getAllByText('users', { exact: true })[0]).toBeInTheDocument()
    expect(screen.getAllByText('roles', { exact: true })[0]).toBeInTheDocument()
  })
})
