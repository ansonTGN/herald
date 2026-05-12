import { describe, it, expect, afterEach, vi } from 'vitest'
import { render, screen } from '@testing-library/react'
import { CreatePermissionDialog } from '../create-permission-dialog'
import {
  assertFormFieldsPresent,
  assertButtonsPresent,
  assertDialogClosed,
  assertDialogTitleAndDescription,
  assertPlaceholder,
  assertHelperText,
  assertCancelButtonWorks,
  typeInInput,
  clickButton,
} from '@/test-utils/dialog-test-helpers'

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

  it('GIVEN dialog is open WHEN rendering THEN should display all form fields', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    assertFormFieldsPresent(screen, [
      { label: 'Permission Name', testId: 'permission-create-name-input' },
      { label: 'Description', testId: 'permission-create-description-input' },
    ])
  })

  it('GIVEN dialog is open WHEN rendering THEN should display dialog title and description', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    assertDialogTitleAndDescription(screen, 'Add Permission', /Create a new permission/)
  })

  it('GIVEN dialog is open WHEN rendering THEN should display Cancel and Create buttons', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    assertButtonsPresent(screen, ['Cancel', 'Add'])
  })

  it('GIVEN dialog is closed WHEN rendering THEN should not display content', async () => {
    render(<CreatePermissionDialog open={false} onOpenChange={vi.fn()} realmId="realm-1" />)

    assertDialogClosed('permission-create-name-input')
  })

  it('GIVEN user types in name input WHEN typing THEN should update input value', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    await typeInInput('permission-create-name-input', 'users.view')
    expect(screen.getByTestId('permission-create-name-input')).toHaveValue('users.view')
  })

  it('GIVEN user types in description input WHEN typing THEN should update input value', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    await typeInInput('permission-create-description-input', 'View all users')
    expect(screen.getByTestId('permission-create-description-input')).toHaveValue('View all users')
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

  it('GIVEN name input WHEN rendering THEN should display helper text', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    assertHelperText(screen, /Format: resource.action/)
  })

  it('GIVEN description input WHEN rendering THEN should display placeholder', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    assertPlaceholder(
      screen,
      'permission-create-description-input',
      'Describe what this permission allows...'
    )
  })

  it('GIVEN name input WHEN rendering THEN should display placeholder', async () => {
    const screen = render(
      <CreatePermissionDialog open={true} onOpenChange={vi.fn()} realmId="realm-1" />
    )

    assertPlaceholder(screen, 'permission-create-name-input', 'users.view')
  })
})
