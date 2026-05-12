import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { EditRoleDialog } from '../edit-role-dialog'
import type { RoleResponse } from '@/lib/api-generated'
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
  updateRole: vi.fn().mockResolvedValue({
    data: {
      id: '1',
      name: 'custom-role',
      description: 'Updated description',
      realmId: 'realm-1',
      clientId: 'admin-web-console',
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
        name: 'custom-role',
        description: 'Updated description',
      },
    }),
  }),
}))

describe('EditRoleDialog', () => {
  const mockRole: RoleResponse = {
    id: '1',
    name: 'custom-role',
    description: 'Custom role description',
    realmId: 'realm-1',
    clientId: 'admin-web-console',
    isBuiltin: false,
  }

  const mockBuiltinRole: RoleResponse = {
    id: '2',
    name: 'realm-admin',
    description: 'Realm administrator',
    realmId: 'realm-1',
    clientId: 'admin-web-console',
    isBuiltin: true,
  }

  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN role data is provided WHEN rendering THEN should load data into form fields', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockRole} realmId="realm-1" />
    )

    assertValue(screen, 'role-edit-name-input', 'custom-role')
    assertValue(screen, 'role-edit-description-input', 'Custom role description')
  })

  it('GIVEN role is builtin WHEN rendering THEN should show warning message', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockBuiltinRole} realmId="realm-1" />
    )

    assertHelperText(screen, /Built-in roles are managed by the platform/)
  })

  it('GIVEN role is builtin WHEN rendering THEN should disable name input', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockBuiltinRole} realmId="realm-1" />
    )

    assertDisabled(screen, 'role-edit-name-input')
  })

  it('GIVEN role is builtin WHEN rendering THEN should disable submit button', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockBuiltinRole} realmId="realm-1" />
    )

    assertDisabled(screen, 'role-edit-submit-button')
  })

  it('GIVEN role is builtin WHEN rendering THEN should allow editing description', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockBuiltinRole} realmId="realm-1" />
    )

    assertEnabled(screen, 'role-edit-description-input')
  })

  it('GIVEN role is custom WHEN rendering THEN should enable name input', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockRole} realmId="realm-1" />
    )

    assertEnabled(screen, 'role-edit-name-input')
  })

  it('GIVEN role is custom WHEN rendering THEN should enable submit button', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockRole} realmId="realm-1" />
    )

    assertEnabled(screen, 'role-edit-submit-button')
  })

  it('GIVEN user clicks Cancel button WHEN clicked THEN should call onOpenChange with false', async () => {
    const handleOpenChange = vi.fn()
    const screen = render(
      <EditRoleDialog
        open={true}
        onOpenChange={handleOpenChange}
        role={mockRole}
        realmId="realm-1"
      />
    )

    assertCancelButtonWorks(screen, handleOpenChange)
  })

  it('GIVEN user types in description input WHEN typing THEN should update input value', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockRole} realmId="realm-1" />
    )

    await typeInInput('role-edit-description-input', 'Updated description', true)
    expect(screen.getByTestId('role-edit-description-input')).toHaveValue('Updated description')
  })

  it('GIVEN role is builtin WHEN rendering THEN should show helper text for name input', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockBuiltinRole} realmId="realm-1" />
    )

    assertHelperText(screen, 'Built-in role names cannot be changed')
  })

  it('GIVEN dialog is open WHEN rendering THEN should display dialog title and description', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockRole} realmId="realm-1" />
    )

    assertDialogTitleAndDescription(screen, 'Edit Role', /Update role details/)
  })

  it('GIVEN dialog is closed WHEN rendering THEN should not display content', async () => {
    render(<EditRoleDialog open={false} onOpenChange={vi.fn()} role={mockRole} realmId="realm-1" />)

    assertDialogClosed('role-edit-name-input')
  })

  it('GIVEN description input WHEN rendering THEN should display placeholder', async () => {
    const screen = render(
      <EditRoleDialog open={true} onOpenChange={vi.fn()} role={mockRole} realmId="realm-1" />
    )

    assertPlaceholder(screen, 'role-edit-description-input', 'Describe what this role is for...')
  })
})
