import { createFileRoute, Outlet } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { useEffect } from 'react'
import { publicConfigQueryOptions } from '@/data/query-options'

export const Route = createFileRoute('/$realmId')({
  component: function RealmRoute() {
    const { realmId } = Route.useParams()
    const { data: publicConfig } = useQuery(publicConfigQueryOptions(realmId))

    useEffect(() => {
      const name = publicConfig?.realmName ?? realmId
      const desc = publicConfig?.realmDescription
      document.title = desc ? `${name} | ${desc}` : name
    }, [realmId, publicConfig?.realmName, publicConfig?.realmDescription])

    return <Outlet />
  },
})
