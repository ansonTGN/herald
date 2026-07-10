import { createFileRoute, Outlet } from '@tanstack/react-router'

export const Route = createFileRoute('/legal')({
  component: LegalLayout,
})

function LegalLayout() {
  return (
    <div className="h-screen overflow-y-auto bg-gradient-to-b from-background to-muted/30">
      <div className="flex min-h-screen items-center justify-center px-4 py-8">
        <Outlet />
      </div>
    </div>
  )
}
