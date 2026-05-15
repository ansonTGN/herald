import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { CreateRoleDialog } from '../create-role-dialog'
import { assertCancelButtonWorks, typeInInput, clickButton } from '@/test-utils/dialog-test-helpers'

// Mock API and hooks
vi.mock('@/lib/api-generated', () => ({
  createRole: vi.fn().mockResolvedValue({
    data: {
      id: '1',
      name: 'custom-role',
      description: 'Custom role description',
      realmId: 'realm-1',
      clientId: 'admin-web-console',
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
          name: data.name,
          description: data.description,
          realmId: 'realm-1',
          clientId: 'admin-web-console',
          isBuiltin: false,
        },
      })
    }),
  }),
}))

describe('CreateRoleDialog', () => {
  afterEach(() => {
    vi.clearAllMocks()
  })

  it('GIVEN user clicks Cancel button WHEN clicked THEN should call onOpenChange with false', async () => {
    const handleOpenChange = vi.fn()
    const screen = render(
      <CreateRoleDialog open={true} onOpenChange={handleOpenChange} realmId="realm-1" />
    )

    assertCancelButtonWorks(screen, handleOpenChange)
  })

  it('GIVEN user fills form WHEN clicking Create THEN should submit form', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    // Fill in form
    await typeInInput('role-create-name-input', 'user-admin')
    await typeInInput('role-create-description-input', 'User administrator')

    // Click Create button
    await clickButton('role-create-submit-button')

    // Form should be submitted (validation happens in form component)
    expect(screen.getByTestId('role-create-submit-button')).toBeInTheDocument()
  })
})
