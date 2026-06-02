import { Sidebar } from '@/components/admin/sidebar'
import { Header } from '@/components/admin/header'
import { Outlet } from '@tanstack/react-router'

export function AdminDashboardLayout() {
  return (
    <div className="flex h-screen bg-background">
      <Sidebar />
      <div className="flex-1 flex flex-col overflow-hidden">
        <Header />
        <main className="flex-1 overflow-y-auto p-6">
          <Outlet />
        </main>
      </div>
    </div>
  )
}
