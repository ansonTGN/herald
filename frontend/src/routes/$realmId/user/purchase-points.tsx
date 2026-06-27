import { m } from '@/paraglide/messages'
import { useState, useEffect, useMemo } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient, useSuspenseQuery } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { AlertCircle, ArrowLeft, ArrowRight, Loader2, Check } from 'lucide-react'
import { createPaymentAttempt, cancelPaymentAttempt } from '@/lib/api-generated'
import type { PaymentAttemptStatusResponse, PurchaseOptionView } from '@/lib/api-generated'
import {
  clientAppsQueryOptions,
  purchaseOptionsQueryOptions,
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
import { formatInvoiceAmount } from '@/lib/invoice-utils'
import { deriveSharedKeyColor } from '@/components/billing/shared-key-color'
import { toast } from 'sonner'

export const Route = createFileRoute('/$realmId/user/purchase-points')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.user.pointsPurchaseVisible, {
      to: '/$realmId/user/points',
      params: { realmId: params.realmId },
    }),
  component: PurchasePointsRoute,
})

type PurchaseStep = 'packages' | 'payment' | 'processing' | 'complete'
type BillingPeriod = 'month' | 'year'

/**
 * Filter the flat purchase-option list to the cards visible in a given period
 * pane.
 *
 * Contract:
 * - `recurring` items appear ONLY in the pane whose `billingPeriod` matches.
 * - `one_time` items are period-agnostic and appear in BOTH panes (a one-time
 *   pack is neither monthly nor annual; hiding it under either toggle would
 *   regress vs. always listing it).
 *
 * Exported pure function so the filtering intent is unit-testable without
 * mounting the Route component.
 */
// eslint-disable-next-line react-refresh/only-export-components -- exported for unit testing
export function selectPeriodPane(
  items: PurchaseOptionView[],
  period: BillingPeriod
): PurchaseOptionView[] {
  return items.filter((item) => {
    // one_time packs are period-agnostic: always visible.
    if (item.billingType !== 'recurring') return true
    return item.billingPeriod === period
  })
}

/**
 * Reason a price card is not purchasable, or null when it is purchasable.
 *
 * A card is disabled when the mapping is not enabled for purchase, or when no
 * payment provider is wired to it (the price exists but cannot be checked out).
 * Returns the `purchase.not_enabled_reason` message args so the caller renders
 * the canonical copy via Paraglide; returns null for purchasable cards so the
 * caller can skip rendering a reason row.
 */
// eslint-disable-next-line react-refresh/only-export-components -- exported for unit testing
export function disabledReason(
  option: PurchaseOptionView
): { key: 'purchase.not_enabled_reason' } | null {
  if (!option.enabled || !option.paymentProvider) {
    return { key: 'purchase.not_enabled_reason' }
  }
  return null
}

function PurchasePointsRoute() {
  const { realmId } = Route.useParams()

  // clientAppId is not in useAuthStore or the route param; resolve it by listing
  // the realm's client apps and taking the first (same pattern as the
  // subscription pages).
  const { data: clientAppsResponse } = useSuspenseQuery(
    clientAppsQueryOptions(realmId, { page: 0, pageSize: 10 })
  )
  const clientAppId = clientAppsResponse.items[0]?.id ?? ''

  if (!clientAppId) {
    return (
      <div className="container" data-testid="purchase-points-page">
        <div className="p-4 text-center text-gray-600" data-testid="no-client-app-message">
          {m['billing.subscription_no_client_app']()}
        </div>
      </div>
    )
  }

  return <PurchasePointsPage realmId={realmId} clientAppId={clientAppId} />
}

function PriceCard({
  option,
  isSelected,
  onSelect,
  periodSuffix,
}: {
  option: PurchaseOptionView
  isSelected: boolean
  onSelect: () => void
  /**
   * Appended to the testid in the Annual pane (`-annual`) so the same priceId
   * can be disambiguated across panes. Empty for Monthly.
   */
  periodSuffix: '' | '-annual'
}) {
  const reason = disabledReason(option)
  const isDisabled = reason !== null
  const color = deriveSharedKeyColor(option.entitlementKey)
  // priceId falls back to mappingId for price-less providers (Creem) so the
  // testid is always stable and non-empty.
  const priceId = option.externalPriceId ?? option.mappingId

  return (
    <Card
      className={`cursor-pointer transition-all ${
        isSelected
          ? 'border-primary ring-2 ring-primary'
          : 'border-muted-foreground/25 hover:border-muted-foreground/50'
      } ${isDisabled ? 'opacity-60' : ''}`}
      onClick={isDisabled ? undefined : onSelect}
      data-testid={`purchase-price-card-${priceId}${periodSuffix}`}
    >
      <CardContent className="p-4">
        <div className="flex w-full items-start justify-between gap-3">
          <div className="flex-1 space-y-1">
            <div className="flex items-center gap-2">
              <span
                className="inline-block h-3 w-3 rounded-full"
                style={
                  color.hue !== 0 ? { backgroundColor: `hsl(${color.hue} 70% 50%)` } : undefined
                }
                aria-hidden
              />
              <div className="font-medium">{option.displayName || option.entitlementKey}</div>
            </div>
            {option.pointsPerPeriod != null && (
              <div className="text-sm text-muted-foreground">
                {option.pointsPerPeriod.toLocaleString()} points
              </div>
            )}
            {option.amount != null && option.currency ? (
              <div className="text-sm font-medium">
                {formatInvoiceAmount(option.amount, option.currency)}
              </div>
            ) : (
              <div className="text-sm text-muted-foreground">{m['purchase.unavailable']()}</div>
            )}
            {isDisabled && reason && (
              <div
                className="text-xs text-muted-foreground"
                data-testid={`purchase-price-card-${priceId}${periodSuffix}-reason`}
              >
                {m[reason.key]()}
              </div>
            )}
          </div>
          {isSelected && (
            <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-primary">
              <Check className="h-4 w-4 text-primary-foreground" />
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  )
}

export function PurchasePointsPage({
  realmId,
  clientAppId,
}: {
  realmId: string
  clientAppId: string
}) {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const user = useAuthStore((state) => state.user)

  // Purchase flow state. Selection is now price-level: the clicked card's
  // mappingId is the checkout target directly. The provider is
  // derived from the selected option, not picked separately.
  const [currentStep, setCurrentStep] = useState<PurchaseStep>('packages')
  const [selectedMappingId, setSelectedMappingId] = useState<string | null>(null)
  const [period, setPeriod] = useState<BillingPeriod>('month')

  // Store actions
  const { setPurchaseState, setPaymentAttempt, clearPurchaseState, canRecover } =
    usePurchaseFlowActions()
  const paymentAttempt = usePaymentAttempt()
  const { attemptId } = paymentAttempt
  const paymentProvider = usePurchaseFlowStore((state) => state.paymentProvider)

  // Fetch purchase options (price-granularity flat list, replaces the former
  // entitlement-key-grouped one-time-mappings source).
  const { data: options, isLoading: optionsLoading } = useQuery(
    purchaseOptionsQueryOptions(realmId, clientAppId)
  )
  // Providers are still fetched so the payment step can render provider context;
  // the selected option's own provider is the one used at submit.
  const { data: providers, isLoading: providersLoading } = useQuery(
    paymentProvidersQueryOptions(realmId)
  )

  const periodPane = useMemo(() => selectPeriodPane(options ?? [], period), [options, period])

  const selectedOption = useMemo(
    () => options?.find((o) => o.mappingId === selectedMappingId),
    [options, selectedMappingId]
  )

  // Poll payment status if attempt exists
  const paymentStatusQuery = useQuery({
    ...paymentAttemptStatusQueryOptions(realmId, attemptId || ''),
    enabled: !!attemptId && currentStep === 'processing',
    refetchInterval: (query) => {
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
      return 2000
    },
  })
  const paymentStatus = paymentStatusQuery.data as PaymentAttemptStatusResponse | undefined

  useEffect(() => {
    const checkRecovery = () => {
      if (canRecover() && attemptId) {
        setCurrentStep('processing')
      }
    }
    checkRecovery()
  }, [attemptId, canRecover])

  useEffect(() => {
    if (paymentStatus) {
      if (paymentStatus.status === 'Succeeded') {
        setCurrentStep('complete')
        clearPurchaseState()
        if (user?.id) {
          queryClient.invalidateQueries({ queryKey: queryKeys.walletsByBucket(realmId) })
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

  // Create payment attempt. The targetType/targetId shape is unchanged from the
  // prior flow (entitlement_mapping + mappingId); only the selection model that
  // feeds mappingId changed (price-level vs entitlement-key-resolved).
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
          paymentProvider: selectedOption?.paymentProvider ?? null,
        })

        setPaymentAttempt(
          data.id,
          'Pending',
          data.paymentContext || { paymentProvider: selectedOption?.paymentProvider ?? '' },
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

  const handleNextStep = () => {
    if (currentStep === 'packages' && selectedMappingId) {
      setCurrentStep('payment')
    } else if (currentStep === 'payment' && selectedMappingId && selectedOption?.paymentProvider) {
      createPaymentMutation.mutate({
        mappingId: selectedMappingId,
        provider: selectedOption.paymentProvider,
      })
    }
  }

  const handlePreviousStep = () => {
    if (currentStep === 'payment') {
      setCurrentStep('packages')
    }
  }

  const handleRetry = () => setCurrentStep('payment')
  const handleCancel = () => cancelPaymentMutation.mutate()
  const handleComplete = () => navigate({ to: `/${realmId}/user/points` })

  // Switch the billing-period toggle. A recurring selection only makes sense
  // in its own pane: switching month<->year with a recurring card selected
  // would otherwise leave Next enabled on a now-hidden card (one_time is
  // period-agnostic, so its selection is retained).
  const switchPeriod = (next: BillingPeriod) => {
    setPeriod(next)
    if (selectedOption?.billingType === 'recurring' && selectedOption.billingPeriod !== next) {
      setSelectedMappingId(null)
    }
  }

  const isNextDisabled = () => {
    if (currentStep === 'packages') return !selectedMappingId
    if (currentStep === 'payment')
      return (
        !selectedMappingId || !selectedOption?.paymentProvider || createPaymentMutation.isPending
      )
    return true
  }

  const renderStepContent = () => {
    switch (currentStep) {
      case 'packages':
        return (
          <div className="space-y-6" data-testid="purchase-step-packages">
            <div>
              <h2 className="text-2xl font-bold">{m['purchase.choose_plan']()}</h2>
            </div>

            {/* Period toggle. Switching re-renders a different card set;
                one_time packs stay in both panes (selectPeriodPane). */}
            <div
              className="inline-flex items-center gap-1 rounded-lg border p-1"
              data-testid="purchase-period-toggle"
              role="group"
              aria-label="Billing period"
            >
              <button
                type="button"
                onClick={() => switchPeriod('month')}
                className={`rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
                  period === 'month' ? 'bg-primary text-primary-foreground' : 'hover:bg-accent'
                }`}
                data-testid="purchase-period-toggle-month"
                aria-pressed={period === 'month'}
              >
                {m['purchase.period_monthly']()}
              </button>
              <button
                type="button"
                onClick={() => switchPeriod('year')}
                className={`flex items-center gap-2 rounded-md px-4 py-1.5 text-sm font-medium transition-colors ${
                  period === 'year' ? 'bg-primary text-primary-foreground' : 'hover:bg-accent'
                }`}
                data-testid="purchase-period-toggle-year"
                aria-pressed={period === 'year'}
              >
                {m['purchase.period_annual']()}
                <span className="rounded-full bg-green-100 px-2 py-0.5 text-xs text-green-700">
                  {m['purchase.save_percent']({ percent: '20' })}
                </span>
              </button>
            </div>

            {optionsLoading ? (
              <div className="flex items-center justify-center py-12">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              </div>
            ) : periodPane.length === 0 ? (
              <div
                className="rounded-lg border border-dashed p-8 text-center text-muted-foreground"
                data-testid="purchase-empty-state"
              >
                {m['points.purchase_no_mappings']()}
              </div>
            ) : (
              <div
                className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3"
                data-testid={`purchase-price-grid-${period}`}
              >
                {periodPane.map((option) => (
                  <PriceCard
                    key={option.mappingId}
                    option={option}
                    isSelected={selectedMappingId === option.mappingId}
                    onSelect={() => setSelectedMappingId(option.mappingId)}
                    periodSuffix={period === 'year' ? '-annual' : ''}
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
                  points: selectedOption?.pointsPerPeriod?.toLocaleString() ?? '',
                  price: selectedOption
                    ? selectedOption.amount != null && selectedOption.currency
                      ? formatInvoiceAmount(selectedOption.amount, selectedOption.currency)
                      : m['points.purchase_price_at_checkout']()
                    : '',
                })}
              </p>
            </div>
            <PaymentMethodSelector
              availableProviders={
                providers?.filter((p) => p.platform === selectedOption?.paymentProvider) ?? []
              }
              selectedProvider={selectedOption?.paymentProvider ?? null}
              onSelect={() => {
                /* provider is derived from the selected price; no-op */
              }}
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
                      : m['purchase.continue_to_checkout']()}
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
