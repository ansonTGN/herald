import { AlertCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useRouter } from '@tanstack/react-router'
import { m } from '@/paraglide/messages'
import { getErrorMessage } from '@/lib/error-utils'

export function UserTableError({ error }: { error: unknown }) {
  const router = useRouter()

  function handleRetry() {
    router.invalidate()
  }

  return (
    <div className="flex flex-col items-center justify-center py-12" data-testid="user-table-error">
      <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
      <h3 className="text-lg font-semibold mb-2">{m['users.error_title']()}</h3>
      <p className="text-sm text-gray-500 mb-4">{getErrorMessage(error)}</p>
      <Button onClick={handleRetry} data-testid="error-retry-button">
        {m['common.retry']()}
      </Button>
    </div>
  )
}
