import { createFileRoute, redirect } from '@tanstack/react-router'

// Legacy path kept as a redirect so existing bookmarks land on the renamed page.
export const Route = createFileRoute('/$realmId/manage/points/default-config')({
  beforeLoad: ({ params }) => {
    throw redirect({ to: '/$realmId/manage/points/registration-rules', params })
  },
})
