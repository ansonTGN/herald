import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/')({
  // NOTE: Authentication and redirect logic is handled by __root.tsx
  // The root route's loader checks for '/' and redirects based on auth status and permissions
  component: () => null,
})
