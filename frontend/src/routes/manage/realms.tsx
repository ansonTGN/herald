import { createFileRoute } from '@tanstack/react-router'
import { realmsSearchSchema } from '@/lib/schemas/search-params'
import { RealmsPage } from '@/routes/$realmId/manage/realms'

export const Route = createFileRoute('/manage/realms')({
  validateSearch: (search) => realmsSearchSchema.parse(search),
  component: RealmsPage,
})
