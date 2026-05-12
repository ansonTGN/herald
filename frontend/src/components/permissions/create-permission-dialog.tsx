import { CreateResourceDialog, type ResourceFormConfig } from '@/components/shared'
import { createPermissionSchema, type CreatePermissionFormData } from '@/lib/schemas/common'
import { createPermissionDefinition } from '@/lib/api-generated'
import { useRealmId } from '@/stores/auth-store'
import { queryKeys } from '@/data/query-options'

interface CreatePermissionDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  realmId?: string // Optional for backward compatibility
}

export function CreatePermissionDialog({
  open,
  onOpenChange,
  realmId: realmIdProp,
}: CreatePermissionDialogProps) {
  const storeRealmId = useRealmId()
  const realmId = realmIdProp ?? storeRealmId
  const config: ResourceFormConfig<CreatePermissionFormData> = {
    schema: createPermissionSchema,
    defaultValues: {
      name: '',
      description: '',
    },
    mutationFn: (data: CreatePermissionFormData) =>
      createPermissionDefinition({
        path: { realmId },
        body: data,
      }),
    getSuccessMessage: (response) => {
      const permission = (response as { data?: { name?: string } }).data
      return `Permission "${permission?.name}" added successfully`
    },
    queryKeysToInvalidate: [queryKeys.permissions(realmId)],
    nameFieldLabel: 'Permission Name',
    nameFieldPlaceholder: 'users.view',
    nameFieldHelpText: 'Format: resource.action (e.g., users.view, roles.manage)',
    nameFieldTestId: 'permission-create-name-input',
    nameInputId: 'permission-name',
    descriptionFieldPlaceholder: 'Describe what this permission allows...',
    descriptionFieldTestId: 'permission-create-description-input',
    descriptionInputId: 'permission-description',
    submitButtonTestId: 'permission-create-submit-button',
    submitButtonText: 'Add',
    submittingButtonText: 'Adding...',
  }

  return (
    <CreateResourceDialog
      open={open}
      onOpenChange={onOpenChange}
      config={config}
      title="Add Permission"
      description="Create a new permission for your realm. Permission names must follow the format resource.action (e.g., users.view)."
    />
  )
}
