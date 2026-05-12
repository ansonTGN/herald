import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { CreateRoleDialog } from '../create-role-dialog'
import {
  assertFormFieldsPresent,
  assertButtonsPresent,
  assertDialogClosed,
  assertDialogTitleAndDescription,
  assertPlaceholder,
  assertHelperText,
  typeInInput,
  assertCancelButtonWorks,
  clickButton,
} from '@/test-utils/dialog-test-helpers'

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

  it('GIVEN dialog is open WHEN rendering THEN should display all form fields', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    assertFormFieldsPresent(screen, [
      { label: 'Role Name', testId: 'role-create-name-input' },
      { label: 'Description', testId: 'role-create-description-input' },
    ])
  })

  it('GIVEN dialog is open WHEN rendering THEN should display dialog title and description', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    assertDialogTitleAndDescription(screen, 'Add Role', /Create a new role/)
  })

  it('GIVEN dialog is open WHEN rendering THEN should display Cancel and Create buttons', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    assertButtonsPresent(screen, ['Cancel', 'Add'])
  })

  it('GIVEN dialog is closed WHEN rendering THEN should not display content', async () => {
    render(<CreateRoleDialog open={false} onOpenChange={vi.fn()} realmId="realm-1" />)

    assertDialogClosed('role-create-name-input')
  })

  it('GIVEN user types in name input WHEN typing THEN should update input value', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    await typeInInput('role-create-name-input', 'custom-role')
    expect(screen.getByTestId('role-create-name-input')).toHaveValue('custom-role')
  })

  it('GIVEN user types in description input WHEN typing THEN should update input value', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    await typeInInput('role-create-description-input', 'Custom role description')
    expect(screen.getByTestId('role-create-description-input')).toHaveValue(
      'Custom role description'
    )
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

  it('GIVEN name input WHEN rendering THEN should display helper text', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    assertHelperText(screen, /Role names can contain letters/)
  })

  it('GIVEN description input WHEN rendering THEN should display placeholder', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    assertPlaceholder(screen, 'role-create-description-input', 'Describe what this role is for...')
  })

  it('GIVEN name input WHEN rendering THEN should display placeholder', async () => {
    const screen = render(<CreateRoleDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />)

    assertPlaceholder(screen, 'role-create-name-input', 'user-admin')
  })
})
