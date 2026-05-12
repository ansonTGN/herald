import { useRealmId } from '@/stores/auth-store'
import { LogOut } from 'lucide-react'
import { useCallback } from 'react'
import { logoutFlow } from '@/lib/auth-utils'

export function ProfileHeader() {
  const realmId = useRealmId()

  const handleLogout = useCallback(async () => {
    await logoutFlow(realmId)
  }, [realmId])

  return (
    <header data-testid="profile-header" className="bg-white border-b border-gray-200 px-6 py-4">
      <div className="flex items-center justify-between">
        <h2 data-testid="profile-heading" className="text-2xl font-bold text-gray-900">
          {realmId} - Profile
        </h2>
        <button
          data-testid="profile-header-logout-button"
          onClick={handleLogout}
          className="flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:text-gray-900 hover:bg-gray-100 rounded-md transition-colors"
        >
          <LogOut className="w-4 h-4" />
          <span>Logout</span>
        </button>
      </div>
    </header>
  )
}
