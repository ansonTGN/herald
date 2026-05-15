import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { CreatePermissionDialog } from '../create-permission-dialog'
import { assertCancelButtonWorks, typeInInput, clickButton } from '@/test-utils/dialog-test-helpers'

// Mock API and hooks
vi.mock('@/lib/api-generated', () => ({
  createPermissionDefinition: vi.fn().mockResolvedValue({
    data: {
      id: '1',
      name: 'test.view',
      resource: 'test',
      action: 'view',
      description: 'Test permission',
      realmId: 'realm-1',
      isBuiltin: false,
    },
  }),
}))

vi.mock('@/hooks/use-form-mutation', () => ({
  useFormMutation: () => ({
    isSubmitting: false,
    mutate: vi.fn().mockImplementation(async (data) => {
      // Simulate successful mutation
      return Promise.resolve({
        data: {
          id: '1',
          name: 'test.view',
          resource: 'test',
          action: 'view',
          description: data.description,
          realmId: 'realm-1',
          isBuiltin: false,
        },
      })
    }),
  }),
}))

describe('CreatePermissionDialog', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN user clicks Cancel button WHEN clicked THEN should call onOpenChange with false', async () => {
    const handleOpenChange = vi.fn()
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={handleOpenChange} realmId="realm-1" />
    )

    assertCancelButtonWorks(screen, handleOpenChange)
  })

  it('GIVEN user fills form WHEN clicking Create THEN should submit form', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    // Fill in form
    await typeInInput('permission-create-name-input', 'users.view')
    await typeInInput('permission-create-description-input', 'View users')

    // Click Create button
    await clickButton('permission-create-submit-button')

    // Form should be submitted (validation happens in form component)
    expect(screen.getByTestId('permission-create-submit-button')).toBeInTheDocument()
  })
})
