import { BaseFormDialog } from '@/components/shared/form-dialog'
import { ProviderConfigForm } from './provider-config-form'
import type { OAuthConfigResponse } from '@/lib/api-generated'
import type { OAuthConfigFormData } from '@/lib/schemas/oauth-config'
import { useOauthSaveMutation } from './oauth-mutations'

interface ProviderConfigDialogProps {
  realmId: string
  open: boolean
  onOpenChange: (open: boolean) => void
  editingConfig?: OAuthConfigResponse | undefined
}

export function ProviderConfigDialog({
  realmId,
  open,
  onOpenChange,
  editingConfig,
}: ProviderConfigDialogProps) {
  const mutation = useOauthSaveMutation({
    realmId,
    editingConfig,
    onSuccess: () => onOpenChange(false),
  })

  const handleSubmit = async (values: OAuthConfigFormData) => {
    console.log('[ProviderConfigDialog] Form submission started', {
      realmId,
      isEditing: !!editingConfig,
      providerType: editingConfig?.providerType,
      values,
    })
    try {
      await mutation.mutateAsync(values)
      console.log('[ProviderConfigDialog] Form submission completed successfully')
    } catch (error) {
      console.error('[ProviderConfigDialog] Form submission failed', error)
      throw error
    }
  }

  return (
    <BaseFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={editingConfig ? 'Edit Provider' : 'Add New Provider'}
      description="Configure OAuth provider settings for third-party login"
      isSubmitting={mutation.isPending}
      data-testid="oauth-config-dialog-title"
    >
      <ProviderConfigForm
        editingConfig={editingConfig}
        onSubmit={handleSubmit}
        isPending={mutation.isPending}
        onCancel={() => onOpenChange(false)}
      />
    </BaseFormDialog>
  )
}
