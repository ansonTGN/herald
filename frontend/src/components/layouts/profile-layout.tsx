import { Outlet } from '@tanstack/react-router'
import { ProfileSidebar } from '@/components/profile/profile-sidebar'
import { ProfileHeader } from '@/components/profile/profile-header'

export function ProfileLayout() {
  return (
    <div className="flex h-screen bg-gray-50">
      <ProfileSidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        <ProfileHeader />
        <main className="flex-1 overflow-y-auto p-6">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
