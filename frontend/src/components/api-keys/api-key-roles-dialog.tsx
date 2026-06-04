import { useState, useEffect, useMemo } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import { RoleSelector } from '@/components/shared/role-selector'
import { useRealmId } from '@/stores/auth-store'
import {
  adminApiKeyRolesQueryOptions,
  updateApiKeyRolesMutation,
  queryKeys,
  rolesQueryOptions,
} from '@/data/query-options'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { m } from '@/paraglide/messages'

interface ApiKeyRolesDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  apiKeyId: string
  apiKeyName: string
}

export function ApiKeyRolesDialog({
  open,
  onOpenChange,
  apiKeyId,
  apiKeyName,
}: ApiKeyRolesDialogProps) {
  const realmId = useRealmId()
  const queryClient = useQueryClient()
  const [selectedRoleIds, setSelectedRoleIds] = useState<string[]>([])
  const [isSaving, setIsSaving] = useState(false)

  // Fetch available roles
  const { data: rolesData, isLoading: isLoadingRoles } = useQuery(rolesQueryOptions(realmId))

  // Fetch API key's current roles
  const { data: apiKeyRolesResponse, isLoading: isLoadingApiKeyRoles } = useQuery({
    ...adminApiKeyRolesQueryOptions(realmId, apiKeyId),
    enabled: open && !!realmId && !!apiKeyId,
  })

  // Derive role IDs from API response (memoized to prevent infinite loops)
  const derivedRoleIds = useMemo(() => {
    if (!apiKeyRolesResponse?.roles) return []
    return apiKeyRolesResponse.roles.map((r) => r.id)
  }, [apiKeyRolesResponse])

  // Sync API key roles data to local state when dialog opens or data loads
  useEffect(() => {
    const hasSameSelection =
      selectedRoleIds.length === derivedRoleIds.length &&
      selectedRoleIds.every((roleId, index) => roleId === derivedRoleIds[index])

    if (open && derivedRoleIds.length > 0 && !hasSameSelection) {
      setSelectedRoleIds(derivedRoleIds)
    }
  }, [open, derivedRoleIds, selectedRoleIds])

  const handleRoleChange = async (newRoleIds: string[]) => {
    setIsSaving(true)

    try {
      await updateApiKeyRolesMutation(realmId, apiKeyId, newRoleIds)

      // Invalidate and refetch
      queryClient.invalidateQueries({ queryKey: queryKeys.apiKeyRoles(realmId, apiKeyId) })
      queryClient.invalidateQueries({ queryKey: queryKeys.apiKeysList(realmId) })

      setSelectedRoleIds(newRoleIds)
    } catch {
      toast.error(m['api_keys.roles_update_failed']())
      // selectedRoleIds is NOT updated on failure -- stays matching server state
    } finally {
      setIsSaving(false)
    }
  }

  const isLoading = isLoadingRoles || isLoadingApiKeyRoles

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md" data-testid="api-key-roles-dialog-content">
        <DialogHeader>
          <DialogTitle data-testid="api-key-roles-dialog-title">
            {m['api_keys.roles_dialog_title']()}
          </DialogTitle>
          <DialogDescription data-testid="api-key-roles-dialog-name">
            {apiKeyName}
          </DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="py-8 text-center text-sm text-muted-foreground">
            {m['api_keys.roles_dialog_loading']()}
          </div>
        ) : (
          <div className="space-y-4">
            <div>
              <label
                className="mb-2 block text-sm font-medium"
                data-testid="api-key-roles-dialog-label"
              >
                {m['api_keys.roles_dialog_assign_label']()}
              </label>
              <RoleSelector
                roles={rolesData ?? []}
                selectedRoleIds={selectedRoleIds}
                onChange={handleRoleChange}
                disabled={isSaving}
                placeholder={m['api_keys.roles_dialog_placeholder']()}
              />
              <p className="mt-2 text-xs text-muted-foreground">
                {m['api_keys.roles_dialog_help']()}
              </p>
            </div>

            <DialogFooter className="border-t pt-4">
              <Button
                type="button"
                onClick={() => onOpenChange(false)}
                disabled={isSaving}
                variant="outline"
                data-testid="api-key-roles-dialog-close"
              >
                {m['api_keys.roles_dialog_close']()}
              </Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
