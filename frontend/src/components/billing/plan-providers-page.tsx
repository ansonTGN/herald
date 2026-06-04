import { useMemo } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { toast } from 'sonner'
import { ArrowLeft } from 'lucide-react'
import { PageHeader } from '@/components/shared'
import { useDialogManager } from '@/hooks/use-dialog-state'
import {
  paymentProvidersQueryOptions,
  queryKeys,
  subscriptionPlanProvidersQueryOptions,
  subscriptionPlanQueryOptions,
} from '@/data/query-options'
import {
  addPaymentProviderToPlan,
  updatePlanPaymentProvider,
  type SubscriptionPlanPaymentProviderResponse,
} from '@/lib/api-generated'
import { type ProviderMappingFormData } from '@/lib/schemas/billing-forms'
import { PlanProviderMappingForm } from './plan-provider-mapping-form'
import { PlanProviderMappingList } from './plan-provider-mapping-list'
import { m } from '@/paraglide/messages'

interface PlanProvidersPageProps {
  realmId: string
  planId: string
}

export function PlanProvidersPage({ realmId, planId }: PlanProvidersPageProps) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const formDialog = useDialogManager<SubscriptionPlanPaymentProviderResponse>()

  const { data: plan } = useQuery(subscriptionPlanQueryOptions(realmId, planId))
  const { data: providers = [] } = useQuery(paymentProvidersQueryOptions(realmId))
  const { data: mappings = [] } = useQuery(subscriptionPlanProvidersQueryOptions(realmId, planId))

  const availableProviders = useMemo(() => {
    const usedProviders = new Set(mappings.map((mapping) => mapping.paymentProvider))
    return providers
      .map((provider) => provider.platform)
      .filter((provider) => !usedProviders.has(provider))
  }, [mappings, providers])

  const createProviderMappingMutation = useMutation({
    mutationFn: async (data: ProviderMappingFormData) => {
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
      toast.success(m['billing.mapping_added']())
      formDialog.close()
      await invalidateProviderQueries()
    },
    onError: (error: Error) => {
      toast.error(m['billing.mapping_add_failed']({ message: error.message }))
    },
  })

  const updateProviderMappingMutation = useMutation({
    mutationFn: async ({
      mappingId,
      data,
    }: {
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
      toast.success(m['billing.mapping_updated']())
      formDialog.close()
      await invalidateProviderQueries()
    },
    onError: (error: Error) => {
      toast.error(m['billing.mapping_update_failed']({ message: error.message }))
    },
  })

  async function invalidateProviderQueries() {
    await queryClient.invalidateQueries({ queryKey: ['subscription-plans', realmId] })
    await queryClient.invalidateQueries({ queryKey: queryKeys.planProviders(realmId, planId) })
    await queryClient.invalidateQueries({ queryKey: queryKeys.featureAvailability(realmId) })
  }

  async function handleMappingSubmit(data: ProviderMappingFormData) {
    if (formDialog.selectedItem) {
      await updateProviderMappingMutation.mutateAsync({
        mappingId: formDialog.selectedItem.id,
        data,
      })
    } else {
      await createProviderMappingMutation.mutateAsync(data)
    }
  }

  return (
    <div className="space-y-6" data-testid="plan-providers-page">
      <PageHeader
        title={m['billing.provider_mapping_title']()}
        subtitle={
          plan?.title ? m['billing.provider_mapping_subtitle']({ title: plan.title }) : undefined
        }
        action={{
          label: m['billing.back_to_billing'](),
          onClick: () =>
            navigate({
              to: '/$realmId/manage/billing',
              params: { realmId },
            }),
          testId: 'back-to-billing-button',
          icon: <ArrowLeft className="mr-2 h-4 w-4" />,
        }}
      />

      <PlanProviderMappingList
        planId={planId}
        realmId={realmId}
        onAdd={() => formDialog.open()}
        onEdit={(mapping) => formDialog.open(mapping)}
      />

      <PlanProviderMappingForm
        open={formDialog.isOpen}
        onOpenChange={formDialog.onOpenChange}
        onSubmit={handleMappingSubmit}
        isSubmitting={
          createProviderMappingMutation.isPending || updateProviderMappingMutation.isPending
        }
        mapping={formDialog.selectedItem ?? undefined}
        realmId={realmId}
        availableProviders={availableProviders}
      />
    </div>
  )
}
