import { useState, useEffect, useMemo } from 'react'
import { useMutation, useQuery } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  createWechatOrder,
  getWechatOrderStatus,
  closeWechatOrder,
  type WechatOrderCreateResponse,
  type WechatOrderStatusResponse,
} from '@/lib/api-generated'

export interface UseWechatPayOptions {
  realmId: string
  planId: string
  clientAppId?: string
  pollInterval?: number // Default 3000ms
}

export interface UseWechatPayResult {
  // State
  status: 'creating' | 'pending' | 'paid' | 'failed' | 'expired' | 'closing'
  orderId: string | null
  codeUrl: string | null
  expiresAt: string | null
  error: Error | null

  // Operations
  createOrder: () => Promise<void>
  cancelPayment: () => Promise<void>
  refreshStatus: () => Promise<void>

  // Computed properties
  isCreating: boolean
  isPending: boolean
  isPaid: boolean
  isFailed: boolean
  isExpired: boolean
  timeRemaining: number // seconds
}

export function useWechatPay({
  realmId,
  planId,
  clientAppId,
  pollInterval = 3000,
}: UseWechatPayOptions): UseWechatPayResult {
  // Create order mutation
  const createOrderMutation = useMutation({
    mutationFn: async () => {
      const result = await createWechatOrder({
        path: { realmId },
        body: { planId, clientAppId },
      })
      return result.data as WechatOrderCreateResponse
    },
    onSuccess: () => {
      toast.success('Order created successfully')
    },
    onError: (error: Error) => {
      toast.error(`Failed to create order: ${error.message}`)
      throw error // Re-throw to ensure error state is set
    },
  })

  // Close order mutation
  const closeOrderMutation = useMutation({
    mutationFn: async (orderId: string) => {
      const result = await closeWechatOrder({
        path: { realmId, orderId },
      })
      return result
    },
    onSuccess: () => {
      toast.success('Order cancelled successfully')
    },
    onError: (error: Error) => {
      toast.error(`Failed to close order: ${error.message}`)
    },
  })

  // Query for order status
  const orderId = createOrderMutation.data?.orderId ?? null

  const statusQuery = useQuery({
    queryKey: ['wechat-order-status', realmId, orderId],
    queryFn: async () => {
      if (!orderId) return null

      const result = await getWechatOrderStatus({
        path: { realmId, orderId },
      })
      return result.data as WechatOrderStatusResponse
    },
    enabled: !!orderId && createOrderMutation.isSuccess,
    refetchInterval: ({ state }) => {
      // Poll only when order is pending and not expired
      const data = state.data as WechatOrderStatusResponse | null
      if (!data) return false

      const now = Date.now()
      const expiresAt = new Date(createOrderMutation.data?.expiresAt || '').getTime()
      const isExpired = now > expiresAt

      if (data.status === 'paid' || data.status === 'closed' || isExpired) {
        return false // Stop polling
      }

      return pollInterval
    },
  })

  const orderData = createOrderMutation.data
  const statusData = statusQuery.data as WechatOrderStatusResponse | null

  // Calculate time remaining directly using Date.now()
  /* eslint-disable react-hooks/purity -- Date.now() is used here for real-time countdown display, purity is ensured by forced re-renders */
  const timeRemaining = orderData?.expiresAt
    ? Math.max(0, Math.floor((new Date(orderData.expiresAt).getTime() - Date.now()) / 1000))
    : 0
  /* eslint-enable react-hooks/purity */

  // Force re-render every second when order is active and not paid/failed
  const [, forceUpdate] = useState({})
  useEffect(() => {
    if (!orderData?.expiresAt || statusData?.status === 'paid' || statusData?.status === 'closed')
      return

    const interval = setInterval(() => forceUpdate({}), 1000)
    return () => clearInterval(interval)
  }, [orderData?.expiresAt, statusData?.status])

  // Determine overall status using derived state from React Query
  const status: UseWechatPayResult['status'] = useMemo(() => {
    if (closeOrderMutation.isPending) return 'closing'
    if (createOrderMutation.isPending) return 'creating'
    if (createOrderMutation.isError) return 'failed'
    if (statusQuery.error && orderId) return 'failed'
    if (!orderId) return 'creating'

    // Check expiration first
    if (timeRemaining === 0) return 'expired'

    // Then check actual order status
    if (statusData?.status === 'paid') return 'paid'
    if (statusData?.status === 'closed') return 'failed'
    if (statusData?.status === 'failed') return 'failed'
    if (statusData?.status === 'pending') return 'pending'

    return 'creating'
  }, [
    closeOrderMutation.isPending,
    createOrderMutation.isPending,
    createOrderMutation.isError,
    orderId,
    timeRemaining,
    statusData?.status,
    statusQuery.error,
  ])

  return {
    // State
    status,
    orderId,
    codeUrl: orderData?.codeUrl ?? null,
    expiresAt: orderData?.expiresAt ?? null,
    error: createOrderMutation.error || statusQuery.error || closeOrderMutation.error || null,

    // Operations
    createOrder: async () => {
      await createOrderMutation.mutateAsync()
    },
    cancelPayment: async () => {
      if (orderId) {
        await closeOrderMutation.mutateAsync(orderId)
      }
    },
    refreshStatus: async () => {
      await statusQuery.refetch()
    },

    // Computed properties
    isCreating: status === 'creating',
    isPending: status === 'pending',
    isPaid: status === 'paid',
    isFailed: status === 'failed',
    isExpired: status === 'expired',
    timeRemaining,
  }
}
