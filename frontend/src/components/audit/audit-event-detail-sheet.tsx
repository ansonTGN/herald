import { useQuery } from '@tanstack/react-query'
import {
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerHeader,
  DrawerTitle,
  DrawerDescription,
} from '@/components/ui/drawer'
import { Badge } from '@/components/ui/badge'
import { Skeleton } from '@/components/ui/skeleton'
import { auditDetailQueryOptions } from '@/data/query-options'
import { formatDateTime } from '@/lib/date-utils'
import { m } from '@/paraglide/messages'

interface AuditEventDetailSheetProps {
  eventId: string | null
  realmId: string
  onClose: () => void
}

function DetailField({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid grid-cols-3 gap-2 py-2">
      <dt className="text-sm font-medium text-muted-foreground">{label}</dt>
      <dd className="col-span-2 text-sm">{children}</dd>
    </div>
  )
}

function DetailSkeleton() {
  return (
    <div className="space-y-3 p-4">
      {Array.from({ length: 8 }).map((_, i) => (
        <div key={i} className="grid grid-cols-3 gap-2 py-2">
          <Skeleton className="h-4 w-20" />
          <Skeleton className="col-span-2 h-4 w-full" />
        </div>
      ))}
    </div>
  )
}

export function AuditEventDetailSheet({ eventId, realmId, onClose }: AuditEventDetailSheetProps) {
  const { data, isLoading, error } = useQuery({
    ...auditDetailQueryOptions(realmId, eventId ?? ''),
    enabled: !!eventId,
  })

  return (
    <Drawer
      direction="right"
      open={!!eventId}
      onOpenChange={(open) => {
        if (!open) onClose()
      }}
    >
      <DrawerContent className="sm:max-w-lg" data-testid="audit-detail-sheet">
        <DrawerHeader>
          <DrawerTitle>{m['audit.detail_title']()}</DrawerTitle>
          <DrawerDescription>
            {data ? m['audit.detail_event_id']({ id: data.id }) : m['audit.detail_loading']()}
          </DrawerDescription>
        </DrawerHeader>

        <div className="overflow-y-auto px-4 pb-4">
          {isLoading && <DetailSkeleton />}

          {error && (
            <div className="py-8 text-center text-destructive" data-testid="audit-detail-error">
              {m['audit.detail_failed_to_load']({ message: error.message })}
            </div>
          )}

          {data && !isLoading && (
            <dl className="divide-y">
              <DetailField label={m['audit.detail_time_label']()}>
                {formatDateTime(data.createdAt)}
              </DetailField>
              <DetailField label={m['audit.detail_actor_label']()}>
                <div>
                  <span>{data.actorName || m['audit.unknown']()}</span>
                  <span className="ml-1 text-muted-foreground">({data.actorId})</span>
                </div>
              </DetailField>
              {data.actorType && (
                <DetailField label={m['audit.detail_actor_type_label']()}>
                  {data.actorType.replace(/_/g, ' ')}
                </DetailField>
              )}
              <DetailField label={m['audit.detail_category_label']()}>
                {data.category.replace(/_/g, ' ')}
              </DetailField>
              <DetailField label={m['audit.detail_action_label']()}>
                <code className="font-mono text-xs">{data.action}</code>
              </DetailField>
              <DetailField label={m['audit.detail_target_label']()}>
                <div>
                  <span>{data.targetName || m['audit.unknown']()}</span>
                  <span className="ml-1 text-muted-foreground">({data.targetId})</span>
                </div>
              </DetailField>
              <DetailField label={m['audit.detail_result_label']()}>
                <Badge
                  variant={data.result === 'success' ? 'default' : 'destructive'}
                  data-testid="audit-detail-result"
                >
                  {data.result}
                </Badge>
              </DetailField>
              {data.ipAddress && (
                <DetailField label={m['audit.detail_ip_address_label']()}>
                  <code className="font-mono text-xs">{data.ipAddress}</code>
                </DetailField>
              )}
              {data.userAgent && (
                <DetailField label={m['audit.detail_user_agent_label']()}>
                  <span className="break-all text-xs">{data.userAgent}</span>
                </DetailField>
              )}
              {data.traceId && (
                <DetailField label={m['audit.detail_trace_id_label']()}>
                  <code className="break-all font-mono text-xs">{data.traceId}</code>
                </DetailField>
              )}
            </dl>
          )}

          {data?.details != null && (
            <div className="mt-4">
              <h4 className="mb-2 text-sm font-medium">{m['audit.detail_details_label']()}</h4>
              <pre
                className="max-h-64 overflow-auto rounded-md bg-muted p-3 text-xs"
                data-testid="audit-detail-json"
              >
                {JSON.stringify(data.details, null, 2)}
              </pre>
            </div>
          )}

          {!data && !isLoading && !error && eventId && (
            <div className="py-8 text-center text-muted-foreground">
              {m['audit.detail_event_not_found']()}
            </div>
          )}
        </div>

        <div className="border-t p-4">
          <DrawerClose asChild>
            <button
              className="w-full rounded-md border px-4 py-2 text-sm"
              data-testid="audit-detail-close-button"
            >
              {m['audit.detail_close']()}
            </button>
          </DrawerClose>
        </div>
      </DrawerContent>
    </Drawer>
  )
}
