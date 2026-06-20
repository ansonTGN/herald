import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Layers, Plus, Search } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Card, CardContent } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { PageHeader } from '@/components/shared'
import {
  creditBucketsListQueryOptions,
  creditBucketDetailQueryOptions,
} from '@/data/query-options'
import { useDeleteCreditBucket } from '@/data/credit-bucket-mutations'
import { m } from '@/paraglide/messages'
import { CreditBucketListItem } from './credit-bucket-list-item'
import { CreditBucketEditor } from './credit-bucket-editor'
import { DeleteBucketConfirmDialog, type BucketInUseError } from './delete-bucket-confirm-dialog'

interface CreditBucketsDirectoryPageProps {
  realmId: string
  /** Selected bucket id from the URL (`validateSearch`). */
  selectedId?: string
  /** Sync selection back to the URL. */
  onSelect: (bucketId: string | undefined) => void
}

/**
 * Bucket directory Master-Detail (design §4.4.2).
 *
 * Left column: search + "New Bucket" + Bucket list. Right column: editor for
 * the selected bucket (or the create form when `selectedId === 'new'`, or an
 * empty-state when nothing is selected). Deletion goes through a destructive
 * AlertDialog that surfaces 409 `bucket_in_use`. NO isDefault control (A4).
 */
export function CreditBucketsDirectoryPage({
  realmId,
  selectedId,
  onSelect,
}: CreditBucketsDirectoryPageProps) {
  const [search, setSearch] = useState('')
  const [deleteOpen, setDeleteOpen] = useState(false)
  const [inUseError, setInUseError] = useState<BucketInUseError | null>(null)

  const listQuery = useQuery(creditBucketsListQueryOptions(realmId))
  const buckets = useMemo(() => listQuery.data ?? [], [listQuery.data])

  const detailQuery = useQuery({
    ...creditBucketDetailQueryOptions(realmId, selectedId ?? ''),
    enabled: !!selectedId && selectedId !== 'new',
  })

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase()
    if (!q) return buckets
    return buckets.filter(
      (b) => b.name.toLowerCase().includes(q) || b.bucketKey.toLowerCase().includes(q),
    )
  }, [buckets, search])

  const deleteMutation = useDeleteCreditBucket(realmId)

  function handleNew() {
    onSelect('new')
  }

  function handleSelect(id: string) {
    onSelect(id)
  }

  function requestDelete() {
    setInUseError(null)
    setDeleteOpen(true)
  }

  async function confirmDelete() {
    if (!selectedId || selectedId === 'new') return
    try {
      await deleteMutation.mutateAsync(selectedId)
      setDeleteOpen(false)
      onSelect(undefined)
    } catch (error) {
      const code = readErrorCode(error)
      if (code === 'bucket_in_use') {
        setInUseError(error as BucketInUseError)
      } else {
        setInUseError({ code })
      }
    }
  }

  const selectedBucketName = useMemo(() => {
    if (selectedId === 'new') return ''
    return buckets.find((b) => b.id === selectedId)?.name ?? ''
  }, [buckets, selectedId])

  return (
    <div className="space-y-6" data-testid="credit-buckets-directory-page">
      <PageHeader
        title={m['credit_buckets.page_title']()}
        subtitle={m['credit_buckets.page_subtitle']()}
        headingTestId="credit-buckets-heading"
        action={{
          label: m['credit_buckets.new_button'](),
          onClick: handleNew,
          testId: 'credit-bucket-new-button',
          icon: <Plus className="mr-2 h-4 w-4" />,
        }}
      />

      <div className="grid gap-6 md:grid-cols-[20rem_1fr]">
        {/* Left column: search + list */}
        <div className="space-y-3">
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={m['credit_buckets.search_placeholder']()}
              className="pl-8"
              data-testid="credit-bucket-search-input"
            />
          </div>

          {listQuery.isLoading ? (
            <LoadingList />
          ) : filtered.length === 0 ? (
            <EmptyState isEmptyRealm={buckets.length === 0} onCreate={handleNew} />
          ) : (
            <div className="space-y-2">
              {filtered.map((bucket) => (
                <CreditBucketListItem
                  key={bucket.id}
                  bucket={bucket}
                  selected={selectedId === bucket.id}
                  onSelect={() => handleSelect(bucket.id)}
                />
              ))}
            </div>
          )}
        </div>

        {/* Right column: editor or empty / create placeholder */}
        <div className="space-y-4">
          {selectedId === 'new' ? (
            <CreditBucketEditor
              realmId={realmId}
              bucket={null}
              formKey="new"
              onSaved={() => onSelect(undefined)}
            />
          ) : detailQuery.isLoading && selectedId ? (
            <Skeleton className="h-64 w-full" />
          ) : selectedId && detailQuery.data ? (
            <>
              <CreditBucketEditor
                realmId={realmId}
                bucket={detailQuery.data}
                formKey={detailQuery.data.id}
                onSaved={() => {
                  listQuery.refetch()
                  detailQuery.refetch()
                }}
              />
              <div className="flex justify-end">
                <Button
                  variant="destructive"
                  onClick={requestDelete}
                  data-testid="credit-bucket-delete-button"
                >
                  {m['credit_buckets.delete_button']()}
                </Button>
              </div>
            </>
          ) : (
            <NoSelection />
          )}
        </div>
      </div>

      <DeleteBucketConfirmDialog
        open={deleteOpen}
        onOpenChange={setDeleteOpen}
        onConfirm={confirmDelete}
        bucketName={selectedBucketName}
        inUseError={inUseError}
        isDeleting={deleteMutation.isPending}
      />
    </div>
  )
}

function LoadingList() {
  return (
    <div className="space-y-2">
      {Array.from({ length: 4 }).map((_, i) => (
        <Skeleton key={i} className="h-16 w-full" />
      ))}
    </div>
  )
}

function EmptyState({
  isEmptyRealm,
  onCreate,
}: {
  isEmptyRealm: boolean
  onCreate: () => void
}) {
  return (
    <Card className="border-dashed" data-testid="credit-buckets-empty-state">
      <CardContent className="flex flex-col items-center justify-center py-12 text-center">
        <Layers className="mb-4 h-12 w-12 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">
          {isEmptyRealm
            ? m['credit_buckets.empty_realm']()
            : m['credit_buckets.empty_search']()}
        </p>
        {isEmptyRealm && (
          <Button className="mt-4" onClick={onCreate} data-testid="credit-bucket-empty-new-button">
            <Plus className="mr-2 h-4 w-4" />
            {m['credit_buckets.new_button']()}
          </Button>
        )}
      </CardContent>
    </Card>
  )
}

function NoSelection() {
  return (
    <Card className="border-dashed" data-testid="credit-buckets-no-selection">
      <CardContent className="flex flex-col items-center justify-center py-16 text-center">
        <Layers className="mb-4 h-12 w-12 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">
          {m['credit_buckets.no_selection']()}
        </p>
      </CardContent>
    </Card>
  )
}

function readErrorCode(error: unknown): string | undefined {
  if (error && typeof error === 'object') {
    if ('code' in error && typeof (error as { code: unknown }).code === 'string') {
      return (error as { code: string }).code
    }
    const content = error as { content?: unknown }
    if (content.content && typeof content.content === 'object') {
      return readErrorCode(content.content)
    }
  }
  return undefined
}
