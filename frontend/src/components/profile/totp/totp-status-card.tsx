import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Shield, Clock, Key } from 'lucide-react'
import { formatDate } from '@/lib/totp-utils'
import { totpStatusQueryOptions } from '@/data/query-options'
import { m } from '@/paraglide/messages'

interface TotpStatusCardProps {
  onEnable: () => void
  onDisable: () => void
  onRegenerate: () => void
}

export function TotpStatusCard({ onEnable, onDisable, onRegenerate }: TotpStatusCardProps) {
  const { data, isLoading } = useQuery(totpStatusQueryOptions)

  if (isLoading) {
    return (
      <Card data-testid="totp-status-card">
        <CardContent className="py-6">
          <p className="text-center text-muted-foreground">{m['profile.totp_loading']()}</p>
        </CardContent>
      </Card>
    )
  }

  if (!data?.enabled) {
    return (
      <Card data-testid="totp-status-card">
        <CardHeader>
          <CardTitle className="flex items-center space-x-2">
            <Shield className="h-5 w-5" />
            <span>{m['profile.totp_title']()}</span>
          </CardTitle>
          <CardDescription>{m['profile.totp_description']()}</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={onEnable} data-testid="totp-enable-button">
            {m['profile.totp_enable_button']()}
          </Button>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card data-testid="totp-status-card-enabled">
      <CardHeader>
        <CardTitle className="flex items-center space-x-2">
          <Shield className="h-5 w-5 text-green-600" />
          <span>{m['profile.totp_title']()}</span>
          <Badge variant="default" className="ml-auto" data-testid="totp-status-badge">
            {m['profile.totp_enabled_badge']()}
          </Badge>
        </CardTitle>
        <CardDescription>{m['profile.totp_protected_description']()}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center space-x-2 text-sm">
          <Clock className="h-4 w-4 text-muted-foreground" />
          <span className="text-muted-foreground">{m['profile.totp_enabled_at']()}</span>
          <span data-testid="totp-enabled-at">{formatDate(data.enabledAt)}</span>
        </div>

        {data.lastVerifiedAt && (
          <div className="flex items-center space-x-2 text-sm">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <span className="text-muted-foreground">{m['profile.totp_last_verified']()}</span>
            <span data-testid="totp-last-verified-at">{formatDate(data.lastVerifiedAt)}</span>
          </div>
        )}

        <div className="flex items-center space-x-2 text-sm">
          <Key className="h-4 w-4 text-muted-foreground" />
          <span className="text-muted-foreground">
            {m['profile.totp_remaining_backup_codes']()}
          </span>
          <Badge variant="outline" data-testid="totp-remaining-backup-codes">
            {data.backupCodes.remaining}
          </Badge>
        </div>

        <div className="flex space-x-2 pt-4">
          <Button variant="outline" onClick={onRegenerate} data-testid="totp-regenerate-button">
            {m['profile.totp_regenerate_codes_button']()}
          </Button>
          <Button variant="destructive" onClick={onDisable} data-testid="totp-disable-button">
            {m['profile.totp_disable_button']()}
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}
