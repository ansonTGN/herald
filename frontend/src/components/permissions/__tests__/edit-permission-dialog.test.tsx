import { describe, it, expect, afterEach, vi } from 'vitest'
import { render } from '@testing-library/react'
import { EditPermissionDialog } from '../edit-permission-dialog'
import type { PermissionResponse } from '@/lib/api-generated'
import {
  assertDisabled,
  assertHelperText,
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
})
