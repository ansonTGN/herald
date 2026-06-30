import { BaseFormDialog } from '@/components/shared/form-dialog'
import { ProviderConfigForm } from './provider-config-form'
import type { OAuthConfigResponse } from '@/lib/api-generated'
import type { OAuthConfigFormData } from '@/lib/schemas/oauth-config'
import { useOauthSaveMutation } from './oauth-mutations'
import { m } from '@/paraglide/messages'

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
      // The mutation's `onError` surfaces the failure toast and logs the error.
      // Do NOT re-throw: TanStack Form's onSubmit has no downstream catcher, so
      // a re-thrown rejection propagates as an unhandled rejection (per vitest
      // config, rejections are expected to be handled in components). Sibling
      // forms share this latent leak; see FE-T06 handoff.
      console.error('[ProviderConfigDialog] Form submission failed', error)
    }
  }

  return (
    <BaseFormDialog
      open={open}
      onOpenChange={onOpenChange}
      title={editingConfig ? m['oauth.edit_provider']() : m['oauth.add_provider_title']()}
      description={m['oauth.configure_description']()}
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
