import { CreateResourceDialog, type ResourceFormConfig } from '@/components/shared'
import { createRoleSchema, type CreateRoleFormData } from '@/lib/schemas/common'
import { createRole } from '@/lib/api-generated'
import { useRealmId } from '@/stores/auth-store'
import { queryKeys } from '@/data/query-options'

interface CreateRoleDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId?: string // Optional for backward compatibility
}

export function CreateRoleDialog({
  open,
  onOpenChange,
  realmId: realmIdProp,
}: CreateRoleDialogProps) {
  const storeRealmId = useRealmId()
  const realmId = realmIdProp ?? storeRealmId
  const config: ResourceFormConfig<CreateRoleFormData> = {
    schema: createRoleSchema,
    defaultValues: {
      name: '',
      description: '',
    },
    mutationFn: (data: CreateRoleFormData) =>
      createRole({
        path: { realmId },
        body: {
          ...data,
          clientId: 'admin-web-console',
        },
      }),
    getSuccessMessage: (response) => {
      const role = (response as { data?: { name?: string } }).data
      return role ? `Role "${role.name}" added successfully` : 'Role added successfully'
    },
    queryKeysToInvalidate: [queryKeys.roles(realmId)],
    nameFieldLabel: 'Role Name',
    nameFieldPlaceholder: 'user-admin',
    nameFieldHelpText:
      'Role names can contain letters, numbers, hyphens, and underscores (e.g., user-admin)',
    nameFieldTestId: 'role-create-name-input',
    nameInputId: 'role-name',
    descriptionFieldPlaceholder: 'Describe what this role is for...',
    descriptionFieldTestId: 'role-create-description-input',
    descriptionInputId: 'role-description',
    submitButtonTestId: 'role-create-submit-button',
    submitButtonText: 'Add',
    submittingButtonText: 'Adding...',
  }

  return (
    <CreateResourceDialog
      open={open}
      onOpenChange={onOpenChange}
      config={config}
      title="Add Role"
      description="Create a new role for your realm. Roles are used to group permissions and assign them to users."
    />
  )
}
