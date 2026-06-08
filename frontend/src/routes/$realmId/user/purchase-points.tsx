import { m } from '@/paraglide/messages'
import { useState, useEffect } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AlertCircle, ArrowLeft, ArrowRight, Loader2, Check } from 'lucide-react'
import { createPaymentAttempt, cancelPaymentAttempt } from '@/lib/api-generated'
import type { PaymentAttemptStatusResponse, OneTimeMappingItem } from '@/lib/api-generated'
import {
  oneTimeMappingsQueryOptions,
  paymentProvidersQueryOptions,
  paymentAttemptStatusQueryOptions,
  queryKeys,
  requireFeature,
} from '@/data/query-options'
import { PaymentMethodSelector } from '@/components/purchase/payment-method-selector'
import { PaymentAttemptStatus } from '@/components/purchase/payment-attempt-status'
import { usePurchaseFlowActions, usePaymentAttempt } from '@/stores/purchase-flow-store'
import { usePurchaseFlowStore } from '@/stores/purchase-flow-store'
import { useAuthStore } from '@/stores/auth-store'
import { formatInvoiceAmount, extractProviderPrice } from '@/lib/invoice-utils'
import { toast } from 'sonner'

export const Route = createFileRoute('/$realmId/user/purchase-points')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.user.pointsPurchaseVisible, {
      to: '/$realmId/user/points',
      params: { realmId: params.realmId },
    }),
  component: PurchasePointsPage,
})

type PurchaseStep = 'packages' | 'payment' | 'processing' | 'complete'

function MappingCard({
  mapping,
  isSelected,
  onSelect,
}: {
  mapping: OneTimeMappingItem
  isSelected: boolean
  onSelect: () => void
}) {
  const priceInfo = extractProviderPrice(mapping.providerProductInfo)
  const hasProvider = !!mapping.paymentProvider

  return (
    <Card
      className={`cursor-pointer transition-all ${
        isSelected
          ? 'border-primary ring-2 ring-primary'
          : 'border-muted-foreground/25 hover:border-muted-foreground/50'
      } ${!hasProvider ? 'opacity-60' : ''}`}
      onClick={hasProvider ? onSelect : undefined}
      data-testid={`mapping-card-${mapping.entitlementKey}`}
    >
      <CardContent className="p-4">
        <div className="flex w-full items-center justify-between">
          <div className="flex-1 space-y-1">
            <div className="font-medium">{mapping.entitlementKey}</div>
            {mapping.pointsPerPeriod != null && (
              <div className="text-sm text-muted-foreground">
                {m['points.purchase_mapping_points']({
                  points: mapping.pointsPerPeriod.toLocaleString(),
                })}
              </div>
            )}
            {mapping.validityDays != null && (
              <div className="text-sm text-muted-foreground">
                {m['points.purchase_mapping_validity']({ days: String(mapping.validityDays) })}
              </div>
            )}
            {priceInfo ? (
              <div className="text-sm font-medium">
                {formatInvoiceAmount(priceInfo.amount, priceInfo.currency)}
              </div>
            ) : (
              <div className="text-sm text-muted-foreground">
                {m['points.purchase_price_at_checkout']()}
              </div>
            )}
            {!hasProvider && (
              <div className="text-xs text-muted-foreground" data-testid="no-provider-hint">
                {m['points.purchase_no_provider']()}
              </div>
            )}
          </div>
          {isSelected && (
            <div className="flex h-6 w-6 items-center justify-center rounded-full bg-primary">
              <Check className="h-4 w-4 text-primary-foreground" />
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  )
}

function PurchasePointsPage() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const user = useAuthStore((state) => state.user)

  // Purchase flow state
  const [currentStep, setCurrentStep] = useState<PurchaseStep>('packages')
  const [selectedMappingId, setSelectedMappingId] = useState<string | null>(null)
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null)

  // Store actions
  const { setPurchaseState, setPaymentAttempt, clearPurchaseState, canRecover } =
    usePurchaseFlowActions()
  const paymentAttempt = usePaymentAttempt()
  const { attemptId } = paymentAttempt
  const paymentProvider = usePurchaseFlowStore((state) => state.paymentProvider)

  // Fetch mappings and providers
  const { data: mappings, isLoading: mappingsLoading } = useQuery(
    oneTimeMappingsQueryOptions(realmId)
  )
  const { data: providers, isLoading: providersLoading } = useQuery(
    paymentProvidersQueryOptions(realmId)
  )

  // Poll payment status if attempt exists
  const paymentStatusQuery = useQuery({
    ...paymentAttemptStatusQueryOptions(realmId, attemptId || ''),
    enabled: !!attemptId && currentStep === 'processing',
    refetchInterval: (query) => {
      // Handle test environment where query might be undefined or a mock
      if (!query || !query.state) {
        return false
      }
      const status = query.state.data as PaymentAttemptStatusResponse | undefined
      if (
        status?.status === 'Succeeded' ||
        status?.status === 'Failed' ||
        status?.status === 'Cancelled' ||
        status?.status === 'Expired'
      ) {
        return false
      }
      return 2000 // Poll every 2 seconds
    },
  })
  const paymentStatus = paymentStatusQuery.data as PaymentAttemptStatusResponse | undefined

  // Handle page refresh recovery
  useEffect(() => {
    const checkRecovery = () => {
      if (canRecover() && attemptId) {
        // Resume polling from existing attempt
        setCurrentStep('processing')
      }
    }

    checkRecovery()
  }, [attemptId, canRecover])

  // Watch payment status changes
  useEffect(() => {
    if (paymentStatus) {
      if (paymentStatus.status === 'Succeeded') {
        setCurrentStep('complete')
        clearPurchaseState()
        if (user?.id) {
          queryClient.invalidateQueries({ queryKey: queryKeys.pointsWallet(realmId, user.id) })
        }
        queryClient.invalidateQueries({ queryKey: queryKeys.purchaseHistory(realmId, {}) })
      } else if (
        paymentStatus.status === 'Failed' ||
        paymentStatus.status === 'Cancelled' ||
        paymentStatus.status === 'Expired'
      ) {
        setCurrentStep('payment')
        clearPurchaseState()
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paymentStatus])

  // Auto-redirect for Stripe/Creem checkout
  const checkoutUrl =
    paymentAttempt.paymentContext?.stripeCheckoutUrl ||
    paymentAttempt.paymentContext?.creemCheckoutUrl ||
    null

  useEffect(() => {
    if (currentStep !== 'processing' || !checkoutUrl) return

    const timer = setTimeout(() => {
      window.location.href = checkoutUrl
    }, 3000)

    return () => clearTimeout(timer)
  }, [currentStep, checkoutUrl])

  // Create payment attempt mutation
  const createPaymentMutation = useMutation({
    mutationFn: async (data: { mappingId: string; provider: string }) => {
      const response = await createPaymentAttempt({
        path: { realmId },
        body: {
          targetType: 'entitlement_mapping',
          targetId: data.mappingId,
          paymentProvider: data.provider,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (data) => {
      if (data && data.id) {
        setPurchaseState({
          realmId,
          userId: user?.id || null,
          targetType: 'entitlement_mapping',
          targetId: selectedMappingId,
          paymentProvider: selectedProvider,
        })

        setPaymentAttempt(
          data.id,
          'Pending',
          data.paymentContext || { paymentProvider: selectedProvider || '' },
          data.expiresAt || new Date(Date.now() + 15 * 60 * 1000).toISOString()
        )

        setCurrentStep('processing')
        queryClient.invalidateQueries({ queryKey: ['payment-attempt-status', realmId, data.id] })
      }
    },
    onError: (error: Error) => {
      toast.error(m['points.purchase_create_failed']({ message: error.message }))
    },
  })

  // Cancel payment mutation
  const cancelPaymentMutation = useMutation({
    mutationFn: async () => {
      if (!attemptId) throw new Error('No payment attempt to cancel')
      const response = await cancelPaymentAttempt({ path: { realmId, attemptId } })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      clearPurchaseState()
      setCurrentStep('payment')
      toast.info(m['points.purchase_cancelled']())
    },
    onError: (error: Error) => {
      toast.error(m['points.purchase_cancel_failed']({ message: error.message }))
    },
  })

  const selectedMapping = mappings?.find((m) => m.id === selectedMappingId)

  const handleNextStep = () => {
    if (currentStep === 'packages' && selectedMappingId) {
      setCurrentStep('payment')
    } else if (currentStep === 'payment' && selectedMappingId && selectedProvider) {
      createPaymentMutation.mutate({ mappingId: selectedMappingId, provider: selectedProvider })
    }
  }

  const handlePreviousStep = () => {
    if (currentStep === 'payment') {
      setCurrentStep('packages')
    }
  }

  const handleRetry = () => {
    setCurrentStep('payment')
  }

  const handleCancel = () => {
    cancelPaymentMutation.mutate()
  }

  const handleComplete = () => {
    navigate({ to: `/${realmId}/user/points` })
  }

  const isNextDisabled = () => {
    if (currentStep === 'packages') return !selectedMappingId
    if (currentStep === 'payment') return !selectedProvider || createPaymentMutation.isPending
    return true
  }

  const renderStepContent = () => {
    switch (currentStep) {
      case 'packages':
        return (
          <div className="space-y-6" data-testid="purchase-step-packages">
            <div>
              <h2 className="text-2xl font-bold">{m['points.purchase_select_package_title']()}</h2>
              <p className="text-muted-foreground">
                {m['points.purchase_select_package_description']()}
              </p>
            </div>
            {mappingsLoading ? (
              <div className="flex items-center justify-center py-12">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              </div>
            ) : !mappings || mappings.length === 0 ? (
              <div
                className="rounded-lg border border-dashed p-8 text-center text-muted-foreground"
                data-testid="purchase-empty-state"
              >
                {m['points.purchase_no_mappings']()}
              </div>
            ) : (
              <div
                className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3"
                data-testid="mapping-cards"
              >
                {mappings.map((mapping) => (
                  <MappingCard
                    key={mapping.id}
                    mapping={mapping}
                    isSelected={selectedMappingId === mapping.id}
                    onSelect={() => setSelectedMappingId(mapping.id)}
                  />
                ))}
              </div>
            )}
          </div>
        )

      case 'payment':
        return (
          <div className="space-y-6" data-testid="purchase-step-payment">
            <div>
              <h2 className="text-2xl font-bold">{m['points.purchase_payment_title']()}</h2>
              <p className="text-muted-foreground">
                {m['points.purchase_payment_description']({
                  points: selectedMapping?.pointsPerPeriod?.toLocaleString() ?? '',
                  price: selectedMapping
                    ? (() => {
                        const priceInfo = extractProviderPrice(selectedMapping.providerProductInfo)
                        return priceInfo
                          ? formatInvoiceAmount(priceInfo.amount, priceInfo.currency)
                          : m['points.purchase_price_at_checkout']()
                      })()
                    : '',
                })}
              </p>
            </div>
            <PaymentMethodSelector
              availableProviders={providers || []}
              selectedProvider={selectedProvider}
              onSelect={setSelectedProvider}
              disabled={providersLoading || createPaymentMutation.isPending}
            />
          </div>
        )

      case 'processing':
        return (
          <div className="space-y-6" data-testid="purchase-step-processing">
            {paymentStatus && attemptId ? (
              <PaymentAttemptStatus
                status={paymentStatus}
                paymentProvider={paymentProvider}
                paymentContext={paymentAttempt.paymentContext}
                onRetry={handleRetry}
                onCancel={handleCancel}
                isRetrying={createPaymentMutation.isPending}
                isCancelling={cancelPaymentMutation.isPending}
              />
            ) : paymentStatusQuery.isError ? (
              <div className="space-y-4" data-testid="payment-status-error">
                <div className="flex items-center gap-3">
                  <AlertCircle className="h-8 w-8 text-destructive" />
                  <div>
                    <h3 className="text-lg font-semibold">
                      {m['points.purchase_processing_load_failed']()}
                    </h3>
                    <p className="text-sm text-muted-foreground">
                      {m['points.purchase_processing_load_failed_description']()}
                    </p>
                  </div>
                </div>
                <div className="flex gap-2">
                  <Button onClick={() => paymentStatusQuery.refetch()}>
                    {m['common.retry']()}
                  </Button>
                  <Button variant="outline" onClick={handleRetry}>
                    {m['points.purchase_processing_back_payment']()}
                  </Button>
                </div>
              </div>
            ) : (
              <div
                className="flex items-center gap-3 text-muted-foreground"
                data-testid="payment-status-loading"
              >
                <Loader2 className="h-8 w-8 animate-spin" />
                <div>
                  <h3 className="text-lg font-semibold text-foreground">
                    {m['points.purchase_processing_checking']()}
                  </h3>
                  <p className="text-sm">{m['points.purchase_processing_waiting']()}</p>
                </div>
              </div>
            )}
          </div>
        )

      case 'complete':
        return (
          <div className="space-y-6 text-center" data-testid="purchase-step-complete">
            <div className="flex justify-center">
              <div className="flex h-16 w-16 items-center justify-center rounded-full bg-green-100">
                <svg
                  className="h-8 w-8 text-green-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
              </div>
            </div>
            <div>
              <h2 className="text-2xl font-bold">{m['points.purchase_complete_title']()}</h2>
              <p className="text-muted-foreground">{m['points.purchase_complete_description']()}</p>
            </div>
            <Button onClick={handleComplete}>{m['points.purchase_view_points']()}</Button>
          </div>
        )

      default:
        return null
    }
  }

  return (
    <div className="container" data-testid="purchase-points-page">
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <CardTitle>{m['points.purchase_page_title']()}</CardTitle>
            <div
              className="flex items-center gap-2 text-sm text-muted-foreground"
              data-testid="purchase-step-indicator"
            >
              <span className={currentStep === 'packages' ? 'font-bold text-primary' : ''}>
                {m['points.purchase_step_select']()}
              </span>
              <span>→</span>
              <span className={currentStep === 'payment' ? 'font-bold text-primary' : ''}>
                {m['points.purchase_step_payment']()}
              </span>
              <span>→</span>
              <span className={currentStep === 'processing' ? 'font-bold text-primary' : ''}>
                {m['points.purchase_step_processing']()}
              </span>
              <span>→</span>
              <span className={currentStep === 'complete' ? 'font-bold text-primary' : ''}>
                {m['points.purchase_step_complete']()}
              </span>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="space-y-6">{renderStepContent()}</div>

          {currentStep === 'packages' || currentStep === 'payment' ? (
            <div className="mt-6 flex justify-between">
              <Button
                variant="outline"
                onClick={handlePreviousStep}
                disabled={currentStep === 'packages'}
                data-testid="purchase-back-button"
              >
                <ArrowLeft className="mr-2 h-4 w-4" />
                {m['points.purchase_back']()}
              </Button>
              <Button
                onClick={handleNextStep}
                disabled={isNextDisabled()}
                data-testid="purchase-next-button"
              >
                {currentStep === 'payment' ? (
                  <>
                    {createPaymentMutation.isPending
                      ? m['points.purchase_processing_button']()
                      : m['points.purchase_complete_button']()}
                    <ArrowRight className="ml-2 h-4 w-4" />
                  </>
                ) : (
                  <>
                    {m['points.purchase_next']()}
                    <ArrowRight className="ml-2 h-4 w-4" />
                  </>
                )}
              </Button>
            </div>
          ) : null}
        </CardContent>
      </Card>
    </div>
  )
}
