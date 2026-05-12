import { AlertCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useRouter } from '@tanstack/react-router'

export function UserTableError({ error }: { error: Error }) {
  const router = useRouter()

  function handleRetry() {
    router.invalidate()
  }

  return (
    <div className="flex flex-col items-center justify-center py-12" data-testid="user-table-error">
      <AlertCircle className="h-12 w-12 text-red-500 mb-4" />
      <h3 className="text-lg font-semibold mb-2">Failed to load users</h3>
      <p className="text-sm text-gray-500 mb-4">{error.message}</p>
      <Button onClick={handleRetry} data-testid="error-retry-button">
        Retry
      </Button>
    </div>
  )
}
