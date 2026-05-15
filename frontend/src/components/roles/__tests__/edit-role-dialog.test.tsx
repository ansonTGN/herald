import { describe, it, expect, afterEach, vi } from 'vitest'
import { render } from '@testing-library/react'
import { EditRoleDialog } from '../edit-role-dialog'
import type { RoleResponse } from '@/lib/api-generated'
import { assertDisabled, assertHelperText } from '@/test-utils/dialog-test-helpers'

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
})
