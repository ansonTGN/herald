import { Outlet } from '@tanstack/react-router'
import { ProfileSidebar } from '@/components/profile/profile-sidebar'
import { ProfileHeader } from '@/components/profile/profile-header'

export function ProfileLayout() {
  return (
    <div className="flex h-screen bg-background">
      <ProfileSidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        <ProfileHeader />
        <main className="flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-2xl px-8 py-10">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  )
}
