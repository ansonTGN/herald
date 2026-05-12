import { UserDropdown } from './user-dropdown'
import { useRealmId } from '@/stores/auth-store'

export function Header() {
  const realmId = useRealmId()

  return (
    <header className="bg-white border-b px-6 py-4" data-testid="admin-header">
      <div className="flex items-center justify-between">
        <div className="flex-1" />
        <div className="flex items-center gap-4">
          <UserDropdown realmId={realmId} />
        </div>
      </div>
    </header>
  )
}
