import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Edit, Trash2, Power, PowerOff } from 'lucide-react'
import { providerConfigsQueryOptions } from '@/data/query-options'
import type { OAuthConfigResponse } from '@/lib/api-generated'
import { PROVIDER_DISPLAY_NAMES } from '@/lib/oauth-provider-constants'
import { getErrorMessage } from '@/lib/error-utils'
import { useOauthToggleMutation, useOauthDeleteMutation } from './oauth-mutations'
import { DeleteProviderDialog } from './delete-provider-dialog'

interface ProviderListProps {
  realmId: string
  onEdit: (config: OAuthConfigResponse) => void
}

export function ProviderList({ realmId, onEdit }: ProviderListProps) {
  const { data: configs, isLoading, error } = useQuery(providerConfigsQueryOptions(realmId))

  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false)
  const [deleteConfig, setDeleteConfig] = useState<OAuthConfigResponse | undefined>(undefined)

  const handleDeleteClick = (config: OAuthConfigResponse) => {
    setDeleteConfig(config)
    setIsDeleteDialogOpen(true)
  }

  const handleDeleteSuccess = () => {
    setIsDeleteDialogOpen(false)
    setDeleteConfig(undefined)
  }

  const toggleMutation = useOauthToggleMutation(realmId)
  const deleteMutation = useOauthDeleteMutation(realmId, handleDeleteSuccess)

  if (isLoading) {
    return <div className="text-center py-12 text-muted-foreground">Loading providers...</div>
  }

  if (error) {
    const errorMessage = getErrorMessage(error)
    return (
      <div className="text-center py-12 text-destructive">
        Error loading providers: {errorMessage}
      </div>
    )
  }

  if (!configs || configs.length === 0) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        <p>No providers configured</p>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {configs.map((config) => (
        <div
          key={config.id}
          data-testid={`provider-row-${config.providerType}`}
          className="flex items-center justify-between p-4 border rounded-lg"
        >
          {/* Provider Info */}
          <div className="flex-1 space-y-1">
            <div className="flex items-center gap-2">
              <span className="font-semibold" data-testid={`provider-name-${config.providerType}`}>
                {PROVIDER_DISPLAY_NAMES[
                  config.providerType as keyof typeof PROVIDER_DISPLAY_NAMES
                ] || config.providerType}
              </span>
              <Badge
                variant={config.enabled ? 'default' : 'secondary'}
                data-testid={`provider-status-${config.providerType}`}
              >
                {config.enabled ? 'Enabled' : 'Disabled'}
              </Badge>
            </div>
            <div
              className="text-sm text-muted-foreground"
              data-testid={`provider-client-id-${config.providerType}`}
            >
              clientId: {config.clientId}
            </div>
            {config.scopes && config.scopes.length > 0 && (
              <div className="flex flex-wrap gap-1">
                {config.scopes.slice(0, 3).map((scope, i) => (
                  <Badge key={i} variant="outline" className="text-xs">
                    {scope}
                  </Badge>
                ))}
                {config.scopes.length > 3 && (
                  <Badge variant="outline" className="text-xs">
                    +{config.scopes.length - 3} more
                  </Badge>
                )}
              </div>
            )}
          </div>

          {/* Actions */}
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => toggleMutation.mutate(config)}
              disabled={toggleMutation.isPending}
              data-testid={`provider-toggle-button-${config.providerType}`}
            >
              {config.enabled ? (
                <>
                  <PowerOff className="mr-2 h-4 w-4" />
                  Disable
                </>
              ) : (
                <>
                  <Power className="mr-2 h-4 w-4" />
                  Enable
                </>
              )}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => onEdit(config)}
              data-testid={`provider-edit-button-${config.providerType}`}
            >
              <Edit className="mr-2 h-4 w-4" />
              Edit
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => handleDeleteClick(config)}
              disabled={deleteMutation.isPending}
              data-testid={`provider-delete-button-${config.providerType}`}
              className="text-destructive hover:text-destructive"
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Delete
            </Button>
          </div>
        </div>
      ))}

      {/* Delete Confirmation Dialog */}
      {deleteConfig && (
        <DeleteProviderDialog
          open={isDeleteDialogOpen}
          onOpenChange={setIsDeleteDialogOpen}
          provider={deleteConfig}
          isPending={deleteMutation.isPending}
          onConfirm={() => deleteMutation.mutate(deleteConfig.providerType)}
        />
      )}
    </div>
  )
}
