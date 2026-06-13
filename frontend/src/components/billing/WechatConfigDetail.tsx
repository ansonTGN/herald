import { Button } from '@/components/ui/button'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { formatDateTimeShort } from '@/lib/date-utils'

export interface WechatConfigFieldsProps {
  config: {
    platform: string
    appId: string
    mchId: string
    serialNo: string
    v3Key: string
    privateKey: string
    notifyUrl: string
    createdAt: string
    updatedAt: string
  }
  showSecrets: boolean
  onShowSecrets: () => void
  onHideSecrets: () => void
}

export function WechatConfigFields({
  config,
  showSecrets,
  onShowSecrets,
  onHideSecrets,
}: WechatConfigFieldsProps) {
  return (
    <div className="space-y-4 p-4">
      <div className="grid grid-cols-2 gap-4">
        <div>
          <div className="text-sm font-medium text-muted-foreground">App ID</div>
          <div className="mt-1" data-testid="app-id-display">
            {config.appId}
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-muted-foreground">Merchant ID</div>
          <div className="mt-1" data-testid="merchant-id-display">
            {config.mchId}
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-muted-foreground">Serial No</div>
          <div className="mt-1" data-testid="serial-no-display">
            {config.serialNo}
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-muted-foreground">API v3 Key</div>
          <div className="mt-1 flex items-center gap-2" data-testid="v3-key-display">
            <code className="text-sm bg-muted px-2 py-1 rounded">{config.v3Key}</code>
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-muted-foreground">Private Key</div>
          <div className="mt-1" data-testid="private-key-display">
            <code className="text-sm bg-muted px-2 py-1 rounded">{config.privateKey}</code>
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-muted-foreground">Notify URL</div>
          <div className="mt-1 break-all text-sm" data-testid="notify-url-display">
            {config.notifyUrl}
          </div>
        </div>
      </div>

      <div className="flex gap-2">
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
  )
}

interface WechatConfigDetailProps {
  config: WechatConfigFieldsProps['config']
  onEdit: () => void
  onDelete: () => void
  onShowSecrets: () => void
  onHideSecrets: () => void
  showSecrets: boolean
}

export function WechatConfigDetail({
  config,
  onEdit,
  onDelete,
  onShowSecrets,
  onHideSecrets,
  showSecrets,
}: WechatConfigDetailProps) {
  return (
    <Card data-testid="wechat-config-detail">
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>WeChat Pay Configuration</CardTitle>
            <CardDescription>Last updated: {formatDateTimeShort(config.updatedAt)}</CardDescription>
          </div>
          <Badge variant="default">Active</Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <WechatConfigFields
          config={config}
          showSecrets={showSecrets}
          onShowSecrets={onShowSecrets}
          onHideSecrets={onHideSecrets}
        />

        <div className="flex gap-2 pt-4 border-t">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onEdit}
            data-testid="edit-wechat-config-button"
          >
            Edit
          </Button>
          <Button
            type="button"
            variant="destructive"
            size="sm"
            onClick={onDelete}
            data-testid="delete-wechat-config-button"
          >
            Delete
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}
