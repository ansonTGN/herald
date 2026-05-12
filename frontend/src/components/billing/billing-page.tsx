import { useState } from 'react'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Plus } from 'lucide-react'
import { type PlanResponse, listPaymentProviders } from '@/lib/api-generated'
import { deletePlan, assignPlanToClientApp, removePlanAssignment } from '@/lib/api-generated'
import { billingPlansQueryOptions, queryKeys } from '@/data/query-options'
import { PlanTable } from './plan-table'
import { PlanAssignmentDialog, type PlanAssignmentSubmitData } from './plan-assignment-dialog'
import { PlanProviderMappingList } from './plan-provider-mapping-list'
import { PlanProviderMappingForm } from './plan-provider-mapping-form'
import { PlanPagination } from './plan-pagination'
import {
  addPaymentProviderToPlan,
  updatePlanPaymentProvider,
  type PlanPaymentProviderResponse,
} from '@/lib/api-generated'
import { toast } from 'sonner'
import { type ProviderMappingFormData } from '@/lib/schemas/billing-forms'
import { ConfirmDeleteDialog, PageHeader } from '@/components/shared'
import type { BillingSearchSchema } from '@/routes/$realmId/manage/billing/index'
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog'

interface BillingPageProps {
  realmId: string
  search: BillingSearchSchema
}

export function BillingPage({ realmId, search }: BillingPageProps) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()

  const { data: plansData, isLoading } = useQuery(
    billingPlansQueryOptions(realmId, {
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
  const [assignmentDialogOpen, setAssignmentDialogOpen] = useState(false)
  const [assigningPlan, setAssigningPlan] = useState<PlanResponse | undefined>(undefined)
  const [deleteConfirmOpen, setDeleteConfirmOpen] = useState(false)

  // Provider mapping dialog states
  const [providerMappingDialogOpen, setProviderMappingDialogOpen] = useState(false)
  const [selectedPlanForMapping, setSelectedPlanForMapping] = useState<string | null>(null)
  const [providerMappingFormOpen, setProviderMappingFormOpen] = useState(false)
  const [editingMapping, setEditingMapping] = useState<PlanPaymentProviderResponse | null>(null)

  // Fetch available payment providers for the realm
  const { data: paymentProvidersResponse } = useQuery({
    queryKey: ['realm-payment-providers', realmId],
    queryFn: async () => {
      const response = await listPaymentProviders({ path: { realmId } })
      if (response.error) throw response.error
      return response.data
    },
    staleTime: 5 * 60 * 1000,
  })

  const availableProviders = (paymentProvidersResponse?.providers ?? []).map((p) => p.platform)

  const deletePlanMutation = useMutation({
    mutationFn: async (planId: string) => {
      const response = await deletePlan({ path: { realmId, planId } })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: async (_, planId) => {
      const plan = plans?.find((p) => p.id === planId)
      toast.success(`Plan "${plan?.title}" deleted successfully`)

      // 先等待数据刷新完成
      await queryClient.invalidateQueries({
        queryKey: ['billing-plans', realmId],
      })

      // 再关闭对话框
      setDeleteConfirmOpen(false)
      setAssigningPlan(undefined)
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
      setAssignmentDialogOpen(false)
      setAssigningPlan(undefined)
      // Invalidate queries and wait for refetch to complete
      await queryClient.invalidateQueries({ queryKey: ['billing-plans', realmId] })
      await queryClient.invalidateQueries({ queryKey: queryKeys.planAssignmentsList(realmId) })
    },
    onError: (error: Error) => {
      toast.error(`Failed to assign plan: ${error.message}`)
    },
  })

  // Create provider mapping mutation
  const createProviderMappingMutation = useMutation({
    mutationFn: async ({ planId, data }: { planId: string; data: ProviderMappingFormData }) => {
      const response = await addPaymentProviderToPlan({
        path: { realmId, planId },
        body: {
          paymentProvider: data.paymentProvider,
          externalProductId: data.externalProductId,
          externalPriceId: data.externalPriceId ?? null,
          enabled: data.enabled ?? true,
        },
      })
      if (response.error) throw new Error(response.error.message)
      return response.data
    },
    onSuccess: async () => {
      toast.success('Payment provider mapping added')
      closeMappingFormAfterSave()
      await invalidateProviderQueries()
    },
    onError: (error: Error) => {
      toast.error(`Failed to add provider mapping: ${error.message}`)
    },
  })

  // Update provider mapping mutation
  const updateProviderMappingMutation = useMutation({
    mutationFn: async ({
      planId,
      mappingId,
      data,
    }: {
      planId: string
      mappingId: string
      data: ProviderMappingFormData
    }) => {
      const response = await updatePlanPaymentProvider({
        path: { realmId, planId, mappingId },
        body: {
          externalProductId: data.externalProductId,
          externalPriceId: data.externalPriceId ?? null,
          enabled: data.enabled,
        },
      })
      if (response.error) throw new Error(response.error.message)
      return response.data
    },
    onSuccess: async () => {
      toast.success('Payment provider mapping updated')
      closeMappingFormAfterSave()
      await invalidateProviderQueries()
    },
    onError: (error: Error) => {
      toast.error(`Failed to update provider mapping: ${error.message}`)
    },
  })

  async function invalidateProviderQueries() {
    // Always invalidate billing plans to refresh payment provider summaries
    await queryClient.invalidateQueries({ queryKey: ['billing-plans', realmId] })

    // Also invalidate the specific plan's provider list if we have a selected plan
    if (selectedPlanForMapping) {
      await queryClient.invalidateQueries({
        queryKey: queryKeys.planProviders(realmId, selectedPlanForMapping),
      })
    }
  }

  function handleCreatePlan() {
    navigate({
      to: '/$realmId/manage/billing/plans/new',
      params: { realmId },
    })
  }

  function handleEditPlan(plan: PlanResponse) {
    navigate({
      to: '/$realmId/manage/billing/plans/$planId/edit',
      params: { realmId, planId: plan.id },
    })
  }

  function handleDeletePlan(plan: PlanResponse) {
    setAssigningPlan(plan)
    setDeleteConfirmOpen(true)
  }

  async function confirmDeletePlan() {
    if (!assigningPlan) return
    await deletePlanMutation.mutateAsync(assigningPlan.id)
  }

  function handleAssignPlan(plan: PlanResponse) {
    setAssigningPlan(plan)
    setAssignmentDialogOpen(true)
  }

  function handleAssignSubmit(data: PlanAssignmentSubmitData) {
    if (!assigningPlan) return
    assignPlanMutation.mutate({ planId: assigningPlan.id, ...data })
  }

  function handleManageProviders(plan: PlanResponse) {
    setSelectedPlanForMapping(plan.id)
    setProviderMappingDialogOpen(true)
  }

  function handleAddMapping() {
    setEditingMapping(null)
    setProviderMappingDialogOpen(false) // Close parent first
    setProviderMappingFormOpen(true) // Then open child
  }

  function handleEditMapping(mapping: PlanPaymentProviderResponse) {
    setEditingMapping(mapping)
    setProviderMappingDialogOpen(false) // Close parent first
    setProviderMappingFormOpen(true) // Then open child
  }

  function closeMappingFormAfterSave() {
    setProviderMappingFormOpen(false)
    setProviderMappingDialogOpen(true) // Reopen parent after save
    setEditingMapping(null)
  }

  function cancelMappingForm() {
    setProviderMappingFormOpen(false)
    setProviderMappingDialogOpen(true) // Reopen parent after cancel
    setEditingMapping(null)
  }

  async function handleMappingSubmit(data: ProviderMappingFormData) {
    if (!selectedPlanForMapping) return

    if (editingMapping) {
      await updateProviderMappingMutation.mutateAsync({
        planId: selectedPlanForMapping,
        mappingId: editingMapping.id,
        data,
      })
    } else {
      await createProviderMappingMutation.mutateAsync({
        planId: selectedPlanForMapping,
        data,
      })
    }
  }

  return (
    <div className="space-y-6" data-testid="billing-page">
      <div className="flex items-start justify-between gap-4">
        <PageHeader
          title="Billing Plans"
          description="Manage subscription plans and their assignments"
        />
        <div className="flex gap-2">
          <Button onClick={handleCreatePlan} data-testid="add-plan-button">
            <Plus className="mr-2 h-4 w-4" />
            Add Plan
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
              No plans found. Click "Add Plan" to create one.
            </div>
          )}
        </CardContent>
      </Card>

      {pagination && <PlanPagination pagination={pagination} onPageChange={handlePageChange} />}

      <PlanAssignmentDialog
        plan={assigningPlan}
        realmId={realmId}
        open={assignmentDialogOpen}
        onOpenChange={setAssignmentDialogOpen}
        onSubmit={handleAssignSubmit}
        isSubmitting={assignPlanMutation.isPending}
      />

      <ConfirmDeleteDialog
        open={deleteConfirmOpen}
        onOpenChange={setDeleteConfirmOpen}
        title="Delete Plan"
        description={`Are you sure you want to delete plan "${assigningPlan?.title}"?`}
        onConfirm={confirmDeletePlan}
        isPending={deletePlanMutation.isPending}
        confirmTestId="confirm-delete-button"
      />

      {/* Provider Mapping Dialog */}
      {selectedPlanForMapping && (
        <Dialog open={providerMappingDialogOpen} onOpenChange={setProviderMappingDialogOpen}>
          <DialogContent
            className="max-w-6xl max-h-[80vh] overflow-y-auto"
            data-testid="provider-mapping-dialog"
          >
            <DialogHeader>
              <DialogTitle>Manage Payment Providers</DialogTitle>
              <DialogDescription>Configure payment providers for this plan</DialogDescription>
            </DialogHeader>
            <PlanProviderMappingList
              planId={selectedPlanForMapping}
              realmId={realmId}
              onAdd={handleAddMapping}
              onEdit={handleEditMapping}
            />
          </DialogContent>
        </Dialog>
      )}

      {/* Provider Mapping Form Dialog */}
      {selectedPlanForMapping && (
        <PlanProviderMappingForm
          open={providerMappingFormOpen}
          onOpenChange={cancelMappingForm}
          onSubmit={handleMappingSubmit}
          isSubmitting={
            createProviderMappingMutation.isPending || updateProviderMappingMutation.isPending
          }
          mapping={editingMapping ?? undefined}
          realmId={realmId}
          availableProviders={availableProviders}
        />
      )}
    </div>
  )
}
