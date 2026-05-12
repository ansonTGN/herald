import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Shield, Clock, Key } from 'lucide-react'
import { formatDate } from '@/lib/totp-utils'
import { totpStatusQueryOptions } from '@/data/query-options'

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
          <p className="text-center text-muted-foreground">Loading TOTP status...</p>
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
            <span>Two-Factor Authentication</span>
          </CardTitle>
          <CardDescription>Add an extra layer of security to your account</CardDescription>
        </CardHeader>
        <CardContent>
          <Button onClick={onEnable} data-testid="totp-enable-button">
            Enable TOTP
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
          <span>Two-Factor Authentication</span>
          <Badge variant="default" className="ml-auto" data-testid="totp-status-badge">
            Enabled
          </Badge>
        </CardTitle>
        <CardDescription>Your account is protected by TOTP</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center space-x-2 text-sm">
          <Clock className="h-4 w-4 text-muted-foreground" />
          <span className="text-muted-foreground">Enabled at:</span>
          <span data-testid="totp-enabled-at">{formatDate(data.enabledAt)}</span>
        </div>

        {data.lastVerifiedAt && (
          <div className="flex items-center space-x-2 text-sm">
            <Clock className="h-4 w-4 text-muted-foreground" />
            <span className="text-muted-foreground">Last verified:</span>
            <span data-testid="totp-last-verified-at">{formatDate(data.lastVerifiedAt)}</span>
          </div>
        )}

        <div className="flex items-center space-x-2 text-sm">
          <Key className="h-4 w-4 text-muted-foreground" />
          <span className="text-muted-foreground">Remaining backup codes:</span>
          <Badge variant="outline" data-testid="totp-remaining-backup-codes">
            {data.backupCodes.remaining}
          </Badge>
        </div>

        <div className="flex space-x-2 pt-4">
          <Button variant="outline" onClick={onRegenerate} data-testid="totp-regenerate-button">
            Regenerate Codes
          </Button>
          <Button variant="destructive" onClick={onDisable} data-testid="totp-disable-button">
            Disable TOTP
          </Button>
        </div>
      </CardContent>
    </Card>
  )
}
