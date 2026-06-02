import { Avatar, AvatarFallback } from '@/components/ui/avatar'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { User, Settings, LogOut } from 'lucide-react'
import { useCallback } from 'react'
import { useAuth } from '@/hooks/use-auth'
import { logoutFlow } from '@/lib/auth-utils'

interface UserDropdownProps {
  realmId: string
}

export function UserDropdown({ realmId }: UserDropdownProps) {
  const { user } = useAuth()

  const handleLogout = useCallback(async () => {
    await logoutFlow(realmId)
  }, [realmId])

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          className="flex items-center gap-2.5 rounded-lg px-2 py-1.5 transition-colors hover:bg-accent"
          data-testid="user-avatar"
        >
          <Avatar className="size-8">
            <AvatarFallback
              data-testid="user-avatar-fallback"
              className="bg-primary/10 text-primary text-xs font-semibold"
            >
              {user?.email?.[0]?.toUpperCase()}
            </AvatarFallback>
          </Avatar>
          <span className="hidden text-sm font-medium sm:block">{user?.email}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56" data-testid="user-dropdown-content">
        <DropdownMenuLabel>
          <div className="flex flex-col space-y-1">
            <p className="text-sm font-medium leading-none" data-testid="user-email-display">
              {user?.email}
            </p>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem asChild data-testid="profile-menu-item">
          <a href={`/${realmId}/user/profile/`}>
            <User className="mr-2 h-4 w-4" />
            <span>Profile</span>
          </a>
        </DropdownMenuItem>
        <DropdownMenuItem asChild data-testid="security-menu-item">
          <a href={`/${realmId}/user/security`}>
            <Settings className="mr-2 h-4 w-4" />
            <span>Security</span>
          </a>
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onClick={handleLogout}
          className="text-destructive focus:text-destructive"
          data-testid="logout-menu-item"
        >
          <LogOut className="mr-2 h-4 w-4" />
          <span>Logout</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
