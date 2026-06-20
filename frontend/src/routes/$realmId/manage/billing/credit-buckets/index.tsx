import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { z } from 'zod'
import { CreditBucketsDirectoryPage } from '@/components/billing/credit-bucket/credit-bucket-directory-page'

const creditBucketsSearchSchema = z.object({
  selected: z.string().optional(),
})

export const Route = createFileRoute('/$realmId/manage/billing/credit-buckets/')({
  validateSearch: creditBucketsSearchSchema,
  component: CreditBucketsIndexRoute,
})

function CreditBucketsIndexRoute() {
  const { realmId } = Route.useParams()
  const search = Route.useSearch()
  const navigate = useNavigate()

  function handleSelect(bucketId: string | undefined) {
    navigate({
      to: '/$realmId/manage/billing/credit-buckets',
      params: { realmId },
      search: (prev: { selected?: string }) =>
        bucketId ? { selected: bucketId } : { ...prev, selected: undefined },
      replace: true,
    })
  }

  return (
    <CreditBucketsDirectoryPage
      realmId={realmId}
      selectedId={search.selected}
      onSelect={handleSelect}
    />
  )
}
