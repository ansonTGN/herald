import { Skeleton } from '@/components/ui/skeleton'

const SKELETON_ROWS = 10

export function UserTableSkeleton() {
  return (
    <div className="space-y-3" data-testid="user-table-skeleton">
      {Array.from({ length: SKELETON_ROWS }).map((_, i) => (
        <Skeleton key={i} className="h-12 w-full" />
      ))}
    </div>
  )
}
