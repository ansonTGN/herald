import { useMemo, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Label } from '@/components/ui/label'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  type SubscriptionPlanAssignmentResponse,
  type SubscriptionPlanResponse,
  type ClientAppItem,
} from '@/lib/api-generated'
import {
  clientAppsQueryOptions,
  subscriptionPlanAssignmentsBatchQueryOptions,
} from '@/data/query-options'

export interface PlanAssignmentSubmitData {
  assignClientAppIds: string[]
  removeAssignments: Array<{ clientAppId: string; assignmentId: string }>
}

interface PlanAssignmentDialogProps {
  plan?: SubscriptionPlanResponse
  realmId: string
  open: boolean
  onOpenChange: (open: boolean) => void
  onSubmit: (data: PlanAssignmentSubmitData) => void
  isSubmitting: boolean
}

export function PlanAssignmentDialog({
  plan,
  realmId,
  open,
  onOpenChange,
  onSubmit,
  isSubmitting,
}: PlanAssignmentDialogProps) {
  const planId = plan?.id ?? ''
  const { data: clientAppsData, isLoading } = useQuery(clientAppsQueryOptions(realmId, {}))
  const clientApps = useMemo<ClientAppItem[]>(() => clientAppsData?.items ?? [], [clientAppsData])
  const clientAppIds = useMemo(() => clientApps.map((app) => app.id), [clientApps])

  const { data: batchAssignments } = useQuery(
    subscriptionPlanAssignmentsBatchQueryOptions(realmId, clientAppIds)
  )

  const [selectedApps, setSelectedApps] = useState<string[]>([])
  const [userModified, setUserModified] = useState(false)
  const [prevOpen, setPrevOpen] = useState(false)

  const assignedAppsWithAssignmentIds = useMemo(
    () =>
      (batchAssignments || [])
        .filter((a: SubscriptionPlanAssignmentResponse) => a.planId === planId)
        .map((a: SubscriptionPlanAssignmentResponse) => ({
          clientAppId: a.clientAppId,
          assignmentId: a.id,
        })),
    [batchAssignments, planId]
  )
  const assignedClientAppIds = useMemo(
    () => assignedAppsWithAssignmentIds.map((item) => item.clientAppId),
    [assignedAppsWithAssignmentIds]
  )

  // Reset state when dialog opens
  if (open !== prevOpen) {
    setPrevOpen(open)
    if (open) {
      setSelectedApps([])
      setUserModified(false)
    }
  }
  const assignmentIdByClientApp = useMemo(
    () =>
      new Map(assignedAppsWithAssignmentIds.map((item) => [item.clientAppId, item.assignmentId])),
    [assignedAppsWithAssignmentIds]
  )

  const selectedAppsForRender = useMemo(() => {
    if (!open) {
      return []
    }
    if (!userModified) {
      return assignedClientAppIds
    }
    return selectedApps
  }, [open, userModified, selectedApps, assignedClientAppIds])

  function handleToggleApp(clientAppId: string) {
    setSelectedApps((prev) => {
      const base = userModified ? prev : assignedClientAppIds
      return base.includes(clientAppId)
        ? base.filter((id) => id !== clientAppId)
        : [...base, clientAppId]
    })
    setUserModified(true)
  }

  function handleSubmit() {
    const initialSelected = new Set(assignedClientAppIds)
    const currentSelected = new Set(selectedAppsForRender)

    const assignClientAppIds = selectedAppsForRender.filter(
      (id: string) => !initialSelected.has(id)
    )
    const removeAssignments = assignedClientAppIds
      .filter((id: string) => !currentSelected.has(id))
      .map((clientAppId: string) => {
        const assignmentId = assignmentIdByClientApp.get(clientAppId)
        return assignmentId ? { clientAppId, assignmentId } : null
      })
      .filter((item): item is { clientAppId: string; assignmentId: string } => item !== null)

    onSubmit({ assignClientAppIds, removeAssignments })
    setSelectedApps([])
  }

  const sortedInitialIds = useMemo(() => [...assignedClientAppIds].sort(), [assignedClientAppIds])
  const sortedSelectedIds = useMemo(
    () => [...selectedAppsForRender].sort(),
    [selectedAppsForRender]
  )
  const hasChanges =
    sortedInitialIds.length !== sortedSelectedIds.length ||
    sortedInitialIds.some((id, index) => id !== sortedSelectedIds[index])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[600px]" data-testid="plan-assignment-dialog">
        <DialogHeader>
          <DialogTitle data-testid="plan-assignment-dialog-title">
            Assign Subscription Plan: {plan?.title}
          </DialogTitle>
          <DialogDescription>Select apps to assign this subscription plan to.</DialogDescription>
        </DialogHeader>

        {isLoading ? (
          <div className="flex items-center justify-center py-8">Loading client apps...</div>
        ) : (
          <ScrollArea className="max-h-[400px]">
            <div className="space-y-3 py-4">
              {clientApps.map((app: ClientAppItem) => (
                <div key={app.id} className="flex items-center space-x-3">
                  <Checkbox
                    id={`app-${app.id}`}
                    checked={selectedAppsForRender.includes(app.id)}
                    onCheckedChange={() => handleToggleApp(app.id)}
                    data-testid={`plan-assignment-checkbox-${app.id}`}
                  />
                  <Label
                    htmlFor={`app-${app.id}`}
                    className="flex-1 cursor-pointer"
                    data-testid={`plan-assignment-label-${app.id}`}
                  >
                    <div>
                      <div className="font-medium">{app.name}</div>
                      <div className="text-sm text-muted-foreground">{app.clientId}</div>
                    </div>
                  </Label>
                </div>
              ))}
            </div>
          </ScrollArea>
        )}

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
            data-testid="plan-assignment-cancel-button"
          >
            Cancel
          </Button>
          <Button
            type="button"
            onClick={handleSubmit}
            disabled={isSubmitting || isLoading || !hasChanges}
            data-testid="plan-assignment-submit-button"
          >
            {isSubmitting ? 'Assigning...' : 'Assign Subscription Plan'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
