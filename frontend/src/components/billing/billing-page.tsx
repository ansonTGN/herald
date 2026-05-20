import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useDialogManager } from '@/hooks/use-dialog-state'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Plus } from 'lucide-react'
import { type SubscriptionPlanResponse } from '@/lib/api-generated'
import { deletePlan, assignPlanToClientApp, removePlanAssignment } from '@/lib/api-generated'
import { subscriptionPlansQueryOptions, queryKeys } from '@/data/query-options'
import { PlanTable } from './plan-table'
import { PlanAssignmentDialog, type PlanAssignmentSubmitData } from './plan-assignment-dialog'
import { ListPagination } from '@/components/shared'
import { toast } from 'sonner'
import { ConfirmDeleteDialog, PageHeader } from '@/components/shared'
import type { BillingSearchSchema } from '@/routes/$realmId/manage/billing/index'

interface BillingPageProps {
  realmId: string
  search: BillingSearchSchema
}

export function BillingPage({ realmId, search }: BillingPageProps) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  const { data: plansData, isLoading } = useQuery(
    subscriptionPlansQueryOptions(realmId, {
      page: search.page,
      pageSize: search.pageSize,
    })
  )

  const plans = plansData?.items ?? []
  const pagination = plansData

  const handlePageChange = (newPage: number) => {
    navigate({
      to: '/$realmId/manage/billing',
      params: { realmId },
      search: { ...search, page: newPage },
    })
  }

  // Dialog states
  const assignDialog = useDialogManager<SubscriptionPlanResponse>()
  const deleteDialog = useDialogManager<SubscriptionPlanResponse>()

  const deletePlanMutation = useMutation({
    mutationFn: async (planId: string) => {
      const response = await deletePlan({ path: { realmId, planId } })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async (_, planId) => {
      const plan = plans?.find((p) => p.id === planId)
      toast.success(`Subscription Plan "${plan?.title}" deleted successfully`)

      // 先等待数据刷新完成
      await queryClient.invalidateQueries({
        queryKey: queryKeys.billingPlans(realmId),
      })
      await queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })

      // 再关闭对话框
      deleteDialog.close()
    },
    onError: (error: Error) => {
      toast.error(`Failed to delete plan: ${error.message}`)
    },
  })

  const assignPlanMutation = useMutation({
    mutationFn: async ({
      planId,
      assignClientAppIds,
      removeAssignments,
    }: {
      planId: string
      assignClientAppIds: string[]
      removeAssignments: Array<{ clientAppId: string; assignmentId: string }>
    }) => {
      const assignRequests = assignClientAppIds.map((clientAppId) =>
        assignPlanToClientApp({
          path: { realmId, clientAppId },
          body: { planId },
        }).then((response) => {
          if (response.error) throw response.error
          return response.data
        })
      )
      const removeRequests = removeAssignments.map(({ clientAppId, assignmentId }) =>
        removePlanAssignment({
          path: { realmId, clientAppId, assignmentId },
        }).then((response) => {
          if (response.error) throw response.error
          return response.data
        })
      )

      await Promise.all([...assignRequests, ...removeRequests])
      return { assigned: assignClientAppIds.length, removed: removeAssignments.length }
    },
    onSuccess: async (_, variables) => {
      const { assignClientAppIds, removeAssignments } = variables
      if (assignClientAppIds.length > 0 && removeAssignments.length > 0) {
        toast.success(
          `Plan assignments updated (${assignClientAppIds.length} assigned, ${removeAssignments.length} removed)`
        )
      } else if (assignClientAppIds.length > 0) {
        toast.success(`Plan assigned to ${assignClientAppIds.length} app(s)`)
      } else {
        toast.success(`Plan assignment removed from ${removeAssignments.length} app(s)`)
      }
      assignDialog.close()
      // Invalidate queries and wait for refetch to complete
      await queryClient.invalidateQueries({ queryKey: queryKeys.billingPlans(realmId) })
      await queryClient.invalidateQueries({ queryKey: queryKeys.planAssignmentsList(realmId) })
      await queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
    },
    onError: (error: Error) => {
      toast.error(`Failed to assign plan: ${error.message}`)
    },
  })

  function handleCreatePlan() {
    navigate({
      to: '/$realmId/manage/billing/plans/new',
      params: { realmId },
    })
  }

  function handleEditPlan(plan: SubscriptionPlanResponse) {
    navigate({
      to: '/$realmId/manage/billing/plans/$planId/edit',
      params: { realmId, planId: plan.id },
    })
  }

  function handleDeletePlan(plan: SubscriptionPlanResponse) {
    deleteDialog.open(plan)
  }

  async function confirmDeletePlan() {
    if (!deleteDialog.selectedItem) return
    await deletePlanMutation.mutateAsync(deleteDialog.selectedItem.id)
  }

  function handleAssignPlan(plan: SubscriptionPlanResponse) {
    assignDialog.open(plan)
  }

  function handleAssignSubmit(data: PlanAssignmentSubmitData) {
    if (!assignDialog.selectedItem) return
    assignPlanMutation.mutate({ planId: assignDialog.selectedItem.id, ...data })
  }

  function handleManageProviders(plan: SubscriptionPlanResponse) {
    navigate({
      to: '/$realmId/manage/billing/plans/$planId/providers',
      params: { realmId, planId: plan.id },
    })
  }

  return (
    <div className="space-y-6" data-testid="billing-page">
      <div className="flex items-start justify-between gap-4">
        <PageHeader title="Subscription Plans" />
        <div className="flex gap-2">
          <Button onClick={handleCreatePlan} data-testid="add-plan-button">
            <Plus className="mr-2 h-4 w-4" />
            Add Subscription Plan
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Plans</CardTitle>
        </CardHeader>
        <CardContent>
          {plans && plans.length > 0 ? (
            <PlanTable
              data={plans}
              isLoading={isLoading}
              onEdit={handleEditPlan}
              onDelete={handleDeletePlan}
              onAssign={handleAssignPlan}
              onManageProviders={handleManageProviders}
            />
          ) : isLoading ? (
            <div className="text-center py-8">Loading...</div>
          ) : (
            <div className="text-center py-8 text-muted-foreground">
              No subscription plans found. Click "Add Subscription Plan" to create one.
            </div>
          )}
        </CardContent>
      </Card>

      {pagination && (
        <ListPagination
          page={pagination.page}
          pageSize={pagination.pageSize}
          total={pagination.total}
          onPageChange={handlePageChange}
          testIdPrefix="plan-pagination"
        />
      )}

      <PlanAssignmentDialog
        plan={assignDialog.selectedItem ?? undefined}
        realmId={realmId}
        open={assignDialog.isOpen}
        onOpenChange={assignDialog.onOpenChange}
        onSubmit={handleAssignSubmit}
        isSubmitting={assignPlanMutation.isPending}
      />

      <ConfirmDeleteDialog
        open={deleteDialog.isOpen}
        onOpenChange={deleteDialog.onOpenChange}
        title="Delete Subscription Plan"
        description={`Are you sure you want to delete subscription plan "${deleteDialog.selectedItem?.title}"?`}
        onConfirm={confirmDeletePlan}
        isPending={deletePlanMutation.isPending}
        confirmTestId="confirm-delete-button"
      />
    </div>
  )
}
