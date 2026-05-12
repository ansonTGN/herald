import { EditResourceDialog, type ResourceFormConfig } from '@/components/shared'
import { updatePermissionSchema, type UpdatePermissionFormData } from '@/lib/schemas/common'
import { updatePermission } from '@/lib/api-generated'
import { useRealmId } from '@/stores/auth-store'
import type { PermissionResponse } from '@/lib/api-generated'
import { queryKeys } from '@/data/query-options'

interface EditPermissionDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  permission: PermissionResponse
  realmId?: string // Optional for backward compatibility
}

export function EditPermissionDialog({
  open,
  onOpenChange,
  permission,
  realmId: realmIdProp,
}: EditPermissionDialogProps) {
  const storeRealmId = useRealmId()
  const realmId = realmIdProp ?? storeRealmId
  const config: ResourceFormConfig<UpdatePermissionFormData> = {
    schema: updatePermissionSchema,
    defaultValues: {
      name: permission.name,
      description: permission.description ?? '',
    },
    mutationFn: (data: UpdatePermissionFormData) =>
      updatePermission({
        path: { realmId, permissionDefinitionId: permission.id },
        body: data,
      }),
    getSuccessMessage: (response) => {
      const perm = (response as { data?: { name?: string } }).data
      return `Permission "${perm?.name}" updated successfully`
    },
    queryKeysToInvalidate: [queryKeys.permissions(realmId)],
    nameFieldLabel: 'Permission Name',
    nameFieldPlaceholder: permission.name,
    descriptionFieldPlaceholder: 'Describe what this permission allows...',
    nameFieldTestId: 'permission-edit-name-input',
    nameInputId: 'edit-permission-name',
    descriptionFieldTestId: 'permission-edit-description-input',
    descriptionInputId: 'edit-permission-description',
    submitButtonTestId: 'permission-edit-submit-button',
    submitButtonText: 'Update',
    submittingButtonText: 'Updating...',
  }

  const builtinProtection = {
    isBuiltin: permission.isBuiltin,
    alertMessage: 'Built-in permissions cannot be modified. You can only change the description.',
    disabledFieldHelpText: 'Built-in permission names cannot be changed',
  }

  return (
    <EditResourceDialog
      open={open}
      onOpenChange={onOpenChange}
      config={config}
      title="Edit Permission"
      description="Update permission details."
      builtinProtection={builtinProtection}
      currentValues={{
        name: permission.name,
        description: permission.description ?? '',
      }}
    />
  )
}
