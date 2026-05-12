import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EditPermissionDialog } from '../edit-permission-dialog'
import type { PermissionResponse } from '@/lib/api-generated'
import {
  assertValue,
  assertDisabled,
  assertEnabled,
  assertDialogClosed,
  assertDialogTitleAndDescription,
  assertPlaceholder,
  assertHelperText,
  assertCancelButtonWorks,
  typeInInput,
} from '@/test-utils/dialog-test-helpers'

// Mock API and hooks
vi.mock('@/lib/api-generated', () => ({
  updatePermission: vi.fn().mockResolvedValue({
    data: {
      id: '1',
      name: 'users.view',
      resource: 'users',
      action: 'view',
      description: 'Updated description',
      realmId: 'realm-1',
      isBuiltin: false,
    },
  }),
}))

vi.mock('@/hooks/use-form-mutation', () => ({
  useFormMutation: () => ({
    isSubmitting: false,
    mutate: vi.fn().mockResolvedValue({
      data: {
        id: '1',
        name: 'users.view',
        description: 'Updated description',
      },
    }),
  }),
}))

describe('EditPermissionDialog', () => {
  const mockPermission: PermissionResponse = {
    id: '1',
    name: 'users.view',
    resource: 'users',
    action: 'view',
    description: 'View users',
    realmId: 'realm-1',
    isBuiltin: false,
  }

  const mockBuiltinPermission: PermissionResponse = {
    id: '2',
    name: 'users.manage',
    resource: 'users',
    action: 'manage',
    description: 'Manage users',
    realmId: 'realm-1',
    isBuiltin: true,
  }

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN permission data is provided WHEN rendering THEN should load data into form fields', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockPermission}
        realmId="realm-1"
      />
    )

    assertValue(screen, 'permission-edit-name-input', 'users.view')
    assertValue(screen, 'permission-edit-description-input', 'View users')
  })

  it('GIVEN permission is builtin WHEN rendering THEN should show warning message', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockBuiltinPermission}
        realmId="realm-1"
      />
    )

    assertHelperText(screen, /Built-in permissions cannot be modified/)
  })

  it('GIVEN permission is builtin WHEN rendering THEN should disable name input', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockBuiltinPermission}
        realmId="realm-1"
      />
    )

    assertDisabled(screen, 'permission-edit-name-input')
  })

  it('GIVEN permission is builtin WHEN rendering THEN should disable submit button', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockBuiltinPermission}
        realmId="realm-1"
      />
    )

    assertDisabled(screen, 'permission-edit-submit-button')
  })

  it('GIVEN permission is builtin WHEN rendering THEN should allow editing description', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockBuiltinPermission}
        realmId="realm-1"
      />
    )

    assertEnabled(screen, 'permission-edit-description-input')
  })

  it('GIVEN permission is custom WHEN rendering THEN should enable name input', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockPermission}
        realmId="realm-1"
      />
    )

    assertEnabled(screen, 'permission-edit-name-input')
  })

  it('GIVEN permission is custom WHEN rendering THEN should enable submit button', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockPermission}
        realmId="realm-1"
      />
    )

    assertEnabled(screen, 'permission-edit-submit-button')
  })

  it('GIVEN user clicks Cancel button WHEN clicked THEN should call onOpenChange with false', async () => {
    const handleOpenChange = vi.fn()
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={handleOpenChange}
        permission={mockPermission}
        realmId="realm-1"
      />
    )

    assertCancelButtonWorks(screen, handleOpenChange)
  })

  it('GIVEN user types in description input WHEN typing THEN should update input value', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockPermission}
        realmId="realm-1"
      />
    )

    await typeInInput('permission-edit-description-input', 'Updated description', true)
    expect(screen.getByTestId('permission-edit-description-input')).toHaveValue(
      'Updated description'
    )
  })

  it('GIVEN permission is builtin WHEN rendering THEN should show helper text for name input', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockBuiltinPermission}
        realmId="realm-1"
      />
    )

    assertHelperText(screen, 'Built-in permission names cannot be changed')
  })

  it('GIVEN dialog is open WHEN rendering THEN should display dialog title and description', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockPermission}
        realmId="realm-1"
      />
    )

    assertDialogTitleAndDescription(screen, 'Edit Permission', /Update permission details/)
  })

  it('GIVEN dialog is closed WHEN rendering THEN should not display content', async () => {
    render(
      <EditPermissionDialog
        open={false}
        onOpenChange={vi.fn()}
        permission={mockPermission}
        realmId="realm-1"
      />
    )

    assertDialogClosed('permission-edit-name-input')
  })

  it('GIVEN description input WHEN rendering THEN should display placeholder', async () => {
    const screen = render(
      <EditPermissionDialog
        open={true}
        onOpenChange={vi.fn()}
        permission={mockPermission}
        realmId="realm-1"
      />
    )

    assertPlaceholder(
      screen,
      'permission-edit-description-input',
      'Describe what this permission allows...'
    )
  })
})
