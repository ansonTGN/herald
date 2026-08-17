import { createFileRoute, redirect } from '@tanstack/react-router'

// Legacy path kept as a redirect so existing bookmarks land on the renamed page.
export const Route = createFileRoute('/manage/points/default-config')({
  beforeLoad: () => {
    throw redirect({ to: '/manage/points/registration-rules' })
  },
})
