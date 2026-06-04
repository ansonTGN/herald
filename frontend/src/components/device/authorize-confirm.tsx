import { Button } from '@/components/ui/button'
import { m } from '@/paraglide/messages'

interface AuthorizeConfirmProps {
  clientAppName: string
  clientAppIconUrl?: string | null
  onConfirm: (approved: boolean) => void
  isLoading: boolean
}

export function AuthorizeConfirm({
  clientAppName,
  clientAppIconUrl,
  onConfirm,
  isLoading,
}: AuthorizeConfirmProps) {
  return (
    <div className="space-y-6">
      <div className="flex items-center gap-3 justify-center">
        {clientAppIconUrl && (
          <img src={clientAppIconUrl} alt={`${clientAppName} icon`} className="w-10 h-10 rounded" />
        )}
        <span className="text-lg font-medium">{clientAppName}</span>
      </div>
      <p className="text-sm text-muted-foreground text-center">
        {m['device.app_requesting_access']()}
      </p>
      <div className="flex gap-3">
        <Button
          onClick={() => onConfirm(true)}
          disabled={isLoading}
          loading={isLoading}
          className="flex-1"
          data-testid="device-authorize-button"
        >
          {m['device.authorize']()}
        </Button>
        <Button
          variant="outline"
          onClick={() => onConfirm(false)}
          disabled={isLoading}
          className="flex-1"
          data-testid="device-deny-button"
        >
          {m['device.deny']()}
        </Button>
      </div>
    </div>
  )
}
