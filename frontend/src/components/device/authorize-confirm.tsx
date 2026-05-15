import { Button } from '@/components/ui/button'

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
        The above application is requesting access to your account.
      </p>
      <div className="flex gap-3">
        <Button
          onClick={() => onConfirm(true)}
          disabled={isLoading}
          loading={isLoading}
          className="flex-1"
          data-testid="device-authorize-button"
        >
          Authorize
        </Button>
        <Button
          variant="outline"
          onClick={() => onConfirm(false)}
          disabled={isLoading}
          className="flex-1"
          data-testid="device-deny-button"
        >
          Deny
        </Button>
      </div>
    </div>
  )
}
