import { useState, useEffect } from 'react'
import { createFileRoute, useNavigate } from '@tanstack/react-router'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ArrowLeft, ArrowRight } from 'lucide-react'
import { createPaymentAttempt, cancelPaymentAttempt } from '@/lib/api-generated'
import type { PaymentAttemptStatusResponse } from '@/lib/api-generated'
import {
  pointsPackagesQueryOptions,
  paymentProvidersQueryOptions,
  paymentAttemptStatusQueryOptions,
  requireFeature,
} from '@/data/query-options'
import { PointsPackageSelector } from '@/components/purchase/points-package-selector'
import { PaymentMethodSelector } from '@/components/purchase/payment-method-selector'
import { PaymentAttemptStatus } from '@/components/purchase/payment-attempt-status'
import { usePurchaseFlowActions, usePaymentAttempt } from '@/stores/purchase-flow-store'
import { usePurchaseFlowStore } from '@/stores/purchase-flow-store'
import { useAuthStore } from '@/stores/auth-store'
import { toast } from 'sonner'
import { formatPrice } from '@/lib/schemas/points-package-forms'

export const Route = createFileRoute('/$realmId/user/purchase-points')({
  beforeLoad: ({ context, params }) =>
    requireFeature(context.queryClient, params.realmId, (f) => f.user.pointsPurchaseVisible, {
      to: '/$realmId/user/points',
      params: { realmId: params.realmId },
    }),
  component: PurchasePointsPage,
})

type PurchaseStep = 'packages' | 'payment' | 'processing' | 'complete'

function PurchasePointsPage() {
  const { realmId } = Route.useParams()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const user = useAuthStore((state) => state.user)

  // Purchase flow state
  const [currentStep, setCurrentStep] = useState<PurchaseStep>('packages')
  const [selectedPackageId, setSelectedPackageId] = useState<string | null>(null)
  const [selectedProvider, setSelectedProvider] = useState<string | null>(null)

  // Store actions
  const { setPurchaseState, setPaymentAttempt, clearPurchaseState, canRecover } =
    usePurchaseFlowActions()
  const paymentAttempt = usePaymentAttempt()
  const { attemptId } = paymentAttempt
  const paymentProvider = usePurchaseFlowStore((state) => state.paymentProvider)

  // Fetch packages and providers
  const { data: packages, isLoading: packagesLoading } = useQuery(
    pointsPackagesQueryOptions(realmId)
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
        queryClient.invalidateQueries({ queryKey: ['points-account', realmId, user?.id] })
        queryClient.invalidateQueries({ queryKey: ['points-package-purchases', realmId] })
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
    mutationFn: async (data: { packageId: string; provider: string }) => {
      const response = await createPaymentAttempt({
        path: { realmId },
        body: {
          targetType: 'points_package',
          targetId: data.packageId,
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
          targetType: 'points_package',
          targetId: selectedPackageId,
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
      toast.error(`Failed to create payment: ${error.message}`)
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
      toast.info('Payment cancelled')
    },
    onError: (error: Error) => {
      toast.error(`Failed to cancel payment: ${error.message}`)
    },
  })

  const selectedPackage = packages?.find((p) => p.id === selectedPackageId)

  const handleNextStep = () => {
    if (currentStep === 'packages' && selectedPackageId) {
      setCurrentStep('payment')
    } else if (currentStep === 'payment' && selectedPackageId && selectedProvider) {
      createPaymentMutation.mutate({ packageId: selectedPackageId, provider: selectedProvider })
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
    if (currentStep === 'packages') return !selectedPackageId
    if (currentStep === 'payment') return !selectedProvider || createPaymentMutation.isPending
    return true
  }

  const renderStepContent = () => {
    switch (currentStep) {
      case 'packages':
        return (
          <div className="space-y-6" data-testid="purchase-step-packages">
            <div>
              <h2 className="text-2xl font-bold">Select Points Package</h2>
              <p className="text-muted-foreground">Choose a points package that fits your needs</p>
            </div>
            <PointsPackageSelector
              packages={packages || []}
              selectedPackageId={selectedPackageId}
              onSelect={setSelectedPackageId}
              disabled={packagesLoading}
            />
          </div>
        )

      case 'payment':
        return (
          <div className="space-y-6" data-testid="purchase-step-payment">
            <div>
              <h2 className="text-2xl font-bold">Select Payment Method</h2>
              <p className="text-muted-foreground">
                Buying {selectedPackage?.points.toLocaleString()} points for{' '}
                {selectedPackage
                  ? formatPrice(selectedPackage.price, selectedPackage.currency)
                  : ''}
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
            {paymentStatus && attemptId && (
              <PaymentAttemptStatus
                status={paymentStatus}
                paymentProvider={paymentProvider}
                paymentContext={paymentAttempt.paymentContext}
                onRetry={handleRetry}
                onCancel={handleCancel}
                isRetrying={createPaymentMutation.isPending}
                isCancelling={cancelPaymentMutation.isPending}
              />
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
              <h2 className="text-2xl font-bold">Purchase Complete!</h2>
              <p className="text-muted-foreground">Your points have been added to your account</p>
            </div>
            <Button onClick={handleComplete}>View My Points</Button>
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
            <CardTitle>Purchase Points</CardTitle>
            <div
              className="flex items-center gap-2 text-sm text-muted-foreground"
              data-testid="purchase-step-indicator"
            >
              <span className={currentStep === 'packages' ? 'font-bold text-primary' : ''}>
                Select Package
              </span>
              <span>→</span>
              <span className={currentStep === 'payment' ? 'font-bold text-primary' : ''}>
                Payment
              </span>
              <span>→</span>
              <span
                className={
                  currentStep === 'processing' || currentStep === 'complete'
                    ? 'font-bold text-primary'
                    : ''
                }
              >
                Complete
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
                Back
              </Button>
              <Button
                onClick={handleNextStep}
                disabled={isNextDisabled()}
                data-testid="purchase-next-button"
              >
                {currentStep === 'payment' ? (
                  <>
                    {createPaymentMutation.isPending ? 'Processing...' : 'Complete Purchase'}
                    <ArrowRight className="ml-2 h-4 w-4" />
                  </>
                ) : (
                  <>
                    Next
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
