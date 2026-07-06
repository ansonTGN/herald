import { createFileRoute, Outlet } from '@tanstack/react-router'
import { useDocumentTitle } from '@/hooks/use-document-title'

export const Route = createFileRoute('/$realmId')({
  component: function RealmRoute() {
    const { realmId } = Route.useParams()
    // Sets document.title to "{page} · {realm}" so browser history entries are
    // meaningful and distinguishable per page. See useDocumentTitle for details.
    useDocumentTitle(realmId)

    return <Outlet />
  },
})
