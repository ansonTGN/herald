import { EditResourceDialog, type ResourceFormConfig } from '@/components/shared'
import { updateRoleSchema, type UpdateRoleFormData } from '@/lib/schemas/common'
import { updateRole } from '@/lib/api-generated'
import { useRealmId } from '@/stores/auth-store'
import type { RoleResponse } from '@/lib/api-generated'
import { queryKeys } from '@/data/query-options'

interface EditRoleDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  role: RoleResponse
  realmId?: string // Optional for backward compatibility
}

export function EditRoleDialog({
  open,
  onOpenChange,
  role,
  realmId: realmIdProp,
}: EditRoleDialogProps) {
  const storeRealmId = useRealmId()
  const realmId = realmIdProp ?? storeRealmId
  const config: ResourceFormConfig<UpdateRoleFormData> = {
    schema: updateRoleSchema,
    defaultValues: {
      name: role.name,
      description: role.description ?? '',
    },
    mutationFn: (data: UpdateRoleFormData) =>
      updateRole({
        path: { realmId, roleId: role.id },
        body: data,
      }),
    getSuccessMessage: (response) => {
      const r = (response as { data?: { name?: string } }).data
      return `Role "${r?.name}" updated successfully`
    },
    queryKeysToInvalidate: [queryKeys.roles(realmId)],
    nameFieldLabel: 'Role Name',
    nameFieldPlaceholder: role.name,
    descriptionFieldPlaceholder: 'Describe what this role is for...',
    nameFieldTestId: 'role-edit-name-input',
    nameInputId: 'edit-role-name',
    descriptionFieldTestId: 'role-edit-description-input',
    descriptionInputId: 'edit-role-description',
    submitButtonTestId: 'role-edit-submit-button',
    submitButtonText: 'Update',
    submittingButtonText: 'Updating...',
  }

  const builtinProtection = {
    isBuiltin: role.isBuiltin,
    alertMessage:
      'Built-in roles are managed by the platform. You can only modify the description.',
    disabledFieldHelpText: 'Built-in role names cannot be changed',
  }

  return (
    <EditResourceDialog
      open={open}
      onOpenChange={onOpenChange}
      config={config}
      title="Edit Role"
      description="Update role details."
      builtinProtection={builtinProtection}
      currentValues={{
        name: role.name,
        description: role.description ?? '',
      }}
    />
  )
}
