import { Users } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { m } from '@/paraglide/messages'

interface UserTableEmptyProps {
  onCreateUser: () => void
}

export function UserTableEmpty({ onCreateUser }: UserTableEmptyProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12" data-testid="user-table-empty">
      <Users className="h-12 w-12 text-gray-400 mb-4" />
      <h3 className="text-lg font-semibold mb-2">{m['users.empty_title']()}</h3>
      <p className="text-sm text-gray-500 mb-4">{m['users.empty_description']()}</p>
      <Button onClick={onCreateUser} data-testid="empty-create-button">
        {m['users.add_button']()}
      </Button>
    </div>
  )
}
