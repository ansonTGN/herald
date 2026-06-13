import { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Edit, Trash2 } from 'lucide-react'
import { formatDateTimeShort } from '@/lib/date-utils'

export interface ShopifyConfigFieldsProps {
  config: {
    shopDomain: string
    apiVersion: string
    webhookEndpoint?: string
    adminAccessToken: string
    storefrontAccessToken: string
    appClientSecret: string
    lastUpdated: string
    enabled: boolean
  }
  showSecrets: boolean
  onShowSecrets: () => void
  onHideSecrets: () => void
}

export function ShopifyConfigFields({
  config,
  showSecrets,
  onShowSecrets,
  onHideSecrets,
}: ShopifyConfigFieldsProps) {
  const maskToken = (token: string, prefix: string) => {
    if (showSecrets) {
      return token
    }
    return `${prefix}***`
  }

  const maskSecret = (secret: string) => {
    if (showSecrets) {
      return secret
    }
    return '***'
  }

  return (
    <div className="space-y-4 p-4">
      <div className="grid grid-cols-2 gap-4">
        <div>
          <div className="text-sm font-medium text-muted-foreground">Shop Domain</div>
          <div className="mt-1" data-testid="shop-domain-display">
            {config.shopDomain}
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-muted-foreground">API Version</div>
          <div className="mt-1" data-testid="api-version-display">
            {config.apiVersion}
          </div>
        </div>

        {config.webhookEndpoint && (
          <div className="col-span-2">
            <div className="text-sm font-medium text-muted-foreground">Webhook Endpoint</div>
            <div className="mt-1 break-all text-sm" data-testid="webhook-endpoint-display">
              {config.webhookEndpoint}
            </div>
          </div>
        )}

        <div>
          <div className="text-sm font-medium text-muted-foreground">Admin Access Token</div>
          <div className="mt-1 flex items-center gap-2" data-testid="admin-access-token-display">
            <code className="text-sm bg-muted px-2 py-1 rounded" data-testid="masked-token-display">
              {maskToken(config.adminAccessToken, 'shpat_')}
            </code>
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-muted-foreground">Storefront Access Token</div>
          <div
            className="mt-1 flex items-center gap-2"
            data-testid="storefront-access-token-display"
          >
            <code className="text-sm bg-muted px-2 py-1 rounded">
              {maskToken(config.storefrontAccessToken, 'shp_')}
            </code>
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-muted-foreground">App Client Secret</div>
          <div className="mt-1 flex items-center gap-2" data-testid="app-client-secret-display">
            <code className="text-sm bg-muted px-2 py-1 rounded">
              {maskSecret(config.appClientSecret)}
            </code>
          </div>
        </div>

        <div className="flex items-end">
          {!showSecrets ? (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={onShowSecrets}
              data-testid="show-secrets-button"
            >
              Show Secrets
            </Button>
          ) : (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={onHideSecrets}
              data-testid="hide-secrets-button"
            >
              Hide Secrets
            </Button>
          )}
        </div>
      </div>
    </div>
  )
}

interface ShopifyConfigDetailProps {
  config: ShopifyConfigFieldsProps['config']
  onEdit: () => void
  onDelete: () => void
}

export function ShopifyConfigDetail({ config, onEdit, onDelete }: ShopifyConfigDetailProps) {
  const [showSecrets, setShowSecrets] = useState(false)
  const [autoHideTimer, setAutoHideTimer] = useState<ReturnType<typeof setTimeout> | null>(null)

  const handleShowSecrets = () => {
    setShowSecrets(true)

    if (autoHideTimer) {
      clearTimeout(autoHideTimer)
    }

    const timer = setTimeout(() => {
      setShowSecrets(false)
    }, 5000)

    setAutoHideTimer(timer)
  }

  const handleHideSecrets = () => {
    setShowSecrets(false)
    if (autoHideTimer) {
      clearTimeout(autoHideTimer)
      setAutoHideTimer(null)
    }
  }

  useEffect(() => {
    return () => {
      if (autoHideTimer) {
        clearTimeout(autoHideTimer)
      }
    }
  }, [autoHideTimer])

  return (
    <Card data-testid="shopify-config-detail">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Shopify Configuration</CardTitle>
            <CardDescription>
              Last updated: {formatDateTimeShort(config.lastUpdated)}
            </CardDescription>
          </div>
          <Badge variant={config.enabled ? 'default' : 'secondary'}>
            {config.enabled ? 'Enabled' : 'Disabled'}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <ShopifyConfigFields
          config={config}
          showSecrets={showSecrets}
          onShowSecrets={handleShowSecrets}
          onHideSecrets={handleHideSecrets}
        />

        <div className="flex gap-2 pt-4 border-t">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onEdit}
            data-testid="edit-shopify-config-button"
          >
            <Edit className="mr-2 h-4 w-4" />
            Edit
          </Button>
          <Button
            type="button"
            variant="destructive"
            size="sm"
            onClick={onDelete}
            data-testid="delete-shopify-config-button"
          >
            <Trash2 className="mr-2 h-4 w-4" />
            Delete
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}
