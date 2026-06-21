import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { redirect } from '@tanstack/react-router'
import {
  listUsers2,
  getUser,
  getUser2,
  listPermissions,
  getPermission,
  listRoles,
  getRole,
  getRolePermissions,
  getUserRoles,
  adminGetUserRoles,
  listRealmsPaginated,
  getRealm2,
  listClientApps,
  getClientApp,
  listOauthConfigs,
  getOauthConfig,
  getPublicConfig,
  getTurnstileStatus,
  getProfile,
  handleGetTotpStatus,
  getSubscriptionForClientApp,
  listWallets,
  getWallet,
  listTransactions,
  getRealmDefaultConfig,
  updateRealmDefaultConfig,
  getPaymentAttemptStatus,
  listPaymentProviders,
  listOneTimeMappings,
  getPurchaseHistory,
  listAuditEvents,
  getAuditEvent,
  getDashboardStats,
  emailStatus,
  listApiKeys,
  getApiKey,
  adminGetApiKeyRoles,
  adminUpdateApiKeyRoles,
  getEntitlementMapping,
  getSubscription,
  listCreditBucketsHandler,
  getCreditBucketHandler,
  getBucketOverviewHandler,
} from '@/lib/api-generated'
import { handleApiResponse } from '@/lib/api-utils'
import type {
  OAuthConfigResponse,
  PaymentAttemptStatusResponse,
  PointsWalletResponse,
  EntitlementMappingListResponse,
  EntitlementMappingResponse,
  SubscriptionListResponse,
  SubscriptionDetailResponse,
  OneTimeMappingExtResponse,
  PurchaseHistoryResponse,
  BucketResponse,
  BucketDetailResponse,
  BucketOverviewResponse,
  ListWalletsByBucketResponse,
} from '@/lib/api-generated'
import type {
  HistoryFilters,
  SingleSubscriptionHistoryResponse,
  GlobalSubscriptionHistoryResponse,
} from '@/types/billing'
import { TIME_CONSTANTS, QUERY_KEYS } from '@/lib/constants'
import { client } from '@/lib/api-generated/client.gen'
import type { InvoiceEligibilitySummary } from '@/lib/api-generated'

// ==================== Enhanced Error Handling ====================

/**
 * Enhanced error handler for API responses with specific HTTP status code handling
 * Provides user-friendly error messages based on HTTP status codes
 */
function handleApiErrorWithStatus(error: unknown): never {
  if (error && typeof error === 'object') {
    // Handle generated API client errors
    if ('error' in error && typeof error.error === 'object') {
      const apiError = error.error as {
        status?: number
        statusCode?: number
        message?: string
        detail?: string
      }

      // Extract status code if available
      const status = apiError.status || apiError.statusCode

      // Extract error message
      const message = apiError.message || apiError.detail || 'An error occurred'

      // Handle specific HTTP status codes
      switch (status) {
        case 400:
          throw new Error(`Bad request: ${message}`)
        case 401:
          throw new Error('Unauthorized, please log in again')
        case 403:
          throw new Error('Insufficient permissions')
        case 404:
          throw new Error('Requested resource not found')
        case 409:
          throw new Error(`Conflict: ${message}`)
        case 422:
          throw new Error(`Validation failed: ${message}`)
        case 429:
          throw new Error('Too many requests, please try later')
        case 500:
          throw new Error('Server error, please try later')
        case 503:
          throw new Error('Service temporarily unavailable')
        default:
          throw new Error(message)
      }
    }

    // Handle standard Error objects
    if (error instanceof Error) {
      throw error
    }
  }

  // Handle unknown errors
  throw new Error('An unknown error occurred')
}

const GC_TIME_5_MIN = TIME_CONSTANTS.FIVE_MINUTES
const GC_TIME_10_MIN = 10 * 60 * 1000
const RETRY_COUNT = 1
const STALE_TIME_2_MIN = TIME_CONSTANTS.TWO_MINUTES
const STALE_TIME_5_MIN = TIME_CONSTANTS.FIVE_MINUTES

const isClientError = (error: unknown): boolean => {
  if (error && typeof error === 'object' && 'message' in error) {
    const message = String(error.message)
    return (
      message.includes('Unauthorized') ||
      message.includes('Insufficient permissions') ||
      message.includes('Bad request')
    )
  }
  return false
}

const clientErrorRetry = (failureCount: number, error: unknown): boolean => {
  if (isClientError(error)) return false
  return failureCount < RETRY_COUNT
}

export const queryKeys = {
  publicConfig: (realmId: string) => [QUERY_KEYS.PUBLIC_CONFIG, realmId] as const,
  realms: (filters: Record<string, unknown>) => [QUERY_KEYS.REALMS, filters] as const,
  realmsList: () => [QUERY_KEYS.REALMS] as const,
  realm: (realmId: string | null) => [QUERY_KEYS.REALM, realmId] as const,
  users: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.USERS, realmId, filters] as const,
  usersList: (realmId: string) => [QUERY_KEYS.USERS, realmId] as const,
  user: (realmId: string, userId: string) => [QUERY_KEYS.USER, realmId, userId] as const,
  adminUser: (realmId: string, userId: string) =>
    [QUERY_KEYS.USER, realmId, 'admin', userId] as const,
  permissions: (realmId: string) => [QUERY_KEYS.PERMISSIONS, realmId] as const,
  permission: (realmId: string, permissionId: string) =>
    [QUERY_KEYS.PERMISSION, realmId, permissionId] as const,
  roles: (realmId: string) => [QUERY_KEYS.ROLES, realmId] as const,
  role: (realmId: string, roleId: string) => [QUERY_KEYS.ROLE, realmId, roleId] as const,
  rolePermissions: (realmId: string, roleId: string) =>
    [QUERY_KEYS.ROLE_PERMISSIONS, realmId, roleId] as const,
  adminUserRoles: (realmId: string, userId: string) =>
    [QUERY_KEYS.ADMIN_USER_ROLES, realmId, userId] as const,
  clientApps: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.CLIENT_APPS, realmId, filters] as const,
  clientAppsList: (realmId: string) => [QUERY_KEYS.CLIENT_APPS, realmId] as const,
  clientApp: (realmId: string, id: string) => [QUERY_KEYS.CLIENT_APP, realmId, id] as const,
  oauthConfigs: (realmId: string) => [QUERY_KEYS.OAUTH_CONFIGS, realmId] as const,
  oauthConfig: (realmId: string, providerType: string) =>
    [QUERY_KEYS.OAUTH_CONFIGS, realmId, providerType] as const,
  profile: () => [QUERY_KEYS.PROFILE] as const,
  totpStatus: () => [QUERY_KEYS.TOTP_STATUS] as const,
  turnstileStatus: (realmId: string) => [QUERY_KEYS.TURNSTILE_STATUS, realmId] as const,
  subscription: (realmId: string, clientAppId: string) =>
    [QUERY_KEYS.SUBSCRIPTION, realmId, clientAppId] as const,
  subscriptionDetails: (realmId: string, subscriptionId: string) =>
    [QUERY_KEYS.SUBSCRIPTION_DETAILS, realmId, subscriptionId] as const,
  subscriptionHistory: (realmId: string, subscriptionId: string) =>
    [QUERY_KEYS.SUBSCRIPTION_HISTORY, realmId, subscriptionId] as const,
  globalSubscriptionHistory: (
    realmId: string,
    filters: HistoryFilters,
    page: number,
    pageSize: number
  ) => [QUERY_KEYS.GLOBAL_SUBSCRIPTION_HISTORY, realmId, filters, page, pageSize] as const,
  userSubscriptions: (realmId: string, clientAppIds: string) =>
    [QUERY_KEYS.USER_SUBSCRIPTIONS, realmId, clientAppIds] as const,
  pointsWallets: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.POINTS_WALLETS, realmId, filters] as const,
  pointsWallet: (realmId: string, userId: string) =>
    [QUERY_KEYS.POINTS_WALLET, realmId, userId] as const,
  pointsTransactions: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.POINTS_TRANSACTIONS, realmId, filters] as const,
  pointsDefaultConfig: (realmId: string) => [QUERY_KEYS.POINTS_DEFAULT_CONFIG, realmId] as const,
  realmConfigs: (realmId: string) => [QUERY_KEYS.REALM_CONFIGS, realmId] as const,
  emailStatus: (realmId: string) => [QUERY_KEYS.EMAIL_STATUS, realmId] as const,
  userRoles: () => [QUERY_KEYS.USER_ROLES] as const,
  oneTimeMappings: (realmId: string) => [QUERY_KEYS.ONE_TIME_MAPPINGS_EXT, realmId] as const,
  purchaseHistory: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.PURCHASE_HISTORY, realmId, filters] as const,
  paymentAttemptStatus: (realmId: string, attemptId: string) =>
    [QUERY_KEYS.PAYMENT_ATTEMPT_STATUS, realmId, attemptId] as const,
  paymentProviders: (realmId: string) => [QUERY_KEYS.PAYMENT_PROVIDERS, realmId] as const,
  audit: (realmId: string, filters?: Record<string, unknown>) =>
    [QUERY_KEYS.AUDIT_EVENTS, realmId, filters ?? {}] as const,
  auditDetail: (realmId: string, eventId: string) =>
    [QUERY_KEYS.AUDIT_EVENT, realmId, eventId] as const,
  dashboardStats: (realmId: string) => [QUERY_KEYS.DASHBOARD_STATS, realmId] as const,
  featureAvailability: (realmId: string) => [QUERY_KEYS.FEATURE_AVAILABILITY, realmId] as const,
  apiKeys: (realmId: string, filters: { page?: number; pageSize?: number }) =>
    [QUERY_KEYS.API_KEYS, realmId, filters] as const,
  apiKeysList: (realmId: string) => [QUERY_KEYS.API_KEYS, realmId] as const,
  apiKey: (realmId: string, id: string) => [QUERY_KEYS.API_KEY, realmId, id] as const,
  apiKeyRoles: (realmId: string, apiKeyId: string) =>
    [QUERY_KEYS.API_KEY_ROLES, realmId, apiKeyId] as const,
  entitlementMappings: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.ENTITLEMENT_MAPPINGS, realmId, filters] as const,
  entitlementMapping: (realmId: string, mappingId: string) =>
    [QUERY_KEYS.ENTITLEMENT_MAPPING, realmId, mappingId] as const,
  subscriptions: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.ADMIN_SUBSCRIPTIONS, realmId, filters] as const,
  adminSubscription: (realmId: string, subscriptionId: string) =>
    [QUERY_KEYS.ADMIN_SUBSCRIPTION, realmId, subscriptionId] as const,
  creditBucketsList: (realmId: string) => [QUERY_KEYS.CREDIT_BUCKETS, realmId] as const,
  creditBucket: (realmId: string, bucketId: string) =>
    [QUERY_KEYS.CREDIT_BUCKETS, realmId, bucketId] as const,
  creditBucketOverview: (realmId: string) => [QUERY_KEYS.CREDIT_BUCKET_OVERVIEW, realmId] as const,
  walletsByBucket: (realmId: string) => [QUERY_KEYS.WALLETS_BY_BUCKET, realmId] as const,
}

// ==================== Public Config ====================

export const publicConfigQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.publicConfig(realmId),
    queryFn: async () => {
      const response = await getPublicConfig({
        path: { realmId },
      })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: 0, // Changed from STALE_TIME_5_MIN to always fetch fresh data
    refetchOnMount: 'always', // Force refetch on every mount
    refetchOnWindowFocus: true,
    gcTime: GC_TIME_10_MIN,
  })

// ==================== Realms ====================

export const realmQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.realm(realmId),
    queryFn: async () => handleApiResponse(await getRealm2({ path: { realmId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export type FeatureAvailabilityResponse = {
  admin: {
    billingVisible: boolean
    billingConfigVisible: boolean
    entitlementMappingsVisible: boolean
    invoicesVisible: boolean
    subscriptionHistoryVisible: boolean
    pointsVisible: boolean
  }
  user: {
    pointsVisible: boolean
    pointsPurchaseVisible: boolean
    subscriptionVisible: boolean
    invoicesVisible: boolean
  }
  facts: {
    hasPaymentProviders: boolean
    hasEntitlementMappings: boolean
    hasEnabledMappings: boolean
    hasOneTimeMappings: boolean
    hasInvoiceSellerConfig: boolean
    hasInvoices: boolean
    hasSubscriptionHistory: boolean
  }
  /**
   * Realm-level invoice eligibility. Surfaced for both admin and user
   * consumers to gate Create/Apply invoice buttons before submit.
   */
  invoiceEligibility: InvoiceEligibilitySummary
}

export const featureAvailabilityQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.featureAvailability(realmId),
    queryFn: async () => {
      const response = await fetch(`/api/realms/${realmId}/feature-availability`)
      return unwrapFetchResponse<FeatureAvailabilityResponse>(response)
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
    gcTime: GC_TIME_5_MIN,
  })

export async function requireFeature(
  queryClient: QueryClient,
  realmId: string,
  check: (features: FeatureAvailabilityResponse) => boolean,
  redirectOptions: { to: string; params?: Record<string, string>; search?: Record<string, unknown> }
) {
  const features = await queryClient.ensureQueryData(featureAvailabilityQueryOptions(realmId))
  if (!check(features)) {
    throw redirect(redirectOptions)
  }
}

export const realmsQueryOptions = (filters: {
  page?: number
  pageSize?: number
  search?: string
  sortBy?: string
  sortOrder?: string
}) =>
  queryOptions({
    queryKey: queryKeys.realms(filters),
    queryFn: async () =>
      handleApiResponse(
        await listRealmsPaginated({
          query: {
            page: filters.page ?? 0,
            pageSize: filters.pageSize ?? 20,
            search: filters.search,
            sortBy: filters.sortBy ?? 'created_at',
            sortOrder: filters.sortOrder ?? 'desc',
          },
        })
      ),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== Users ====================

export const usersQueryOptions = (
  realmId: string,
  filters: {
    page?: number
    pageSize?: number
    email?: string
    status?: string
  }
) =>
  queryOptions({
    queryKey: queryKeys.users(realmId, filters),
    queryFn: async () =>
      handleApiResponse(
        await listUsers2({
          path: { realmId },
          query: {
            page: filters.page ?? 0,
            pageSize: filters.pageSize ?? 20,
            email: filters.email,
            // status is supported by backend but not yet in generated types
            ...(filters.status ? { status: Number(filters.status) } : {}),
          } as { page?: number; pageSize?: number; email?: string; status?: number },
        })
      ),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const userQueryOptions = (realmId: string, userId: string) =>
  queryOptions({
    queryKey: queryKeys.user(realmId, userId),
    queryFn: async () => handleApiResponse(await getUser({ path: { realmId, userId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const adminUserQueryOptions = (realmId: string, userId: string) =>
  queryOptions({
    queryKey: queryKeys.adminUser(realmId, userId),
    queryFn: async () => handleApiResponse(await getUser2({ path: { realmId, userId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== Permissions ====================

export const permissionsQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.permissions(realmId),
    queryFn: async () => handleApiResponse(await listPermissions({ path: { realmId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const permissionQueryOptions = (realmId: string, permissionId: string) =>
  queryOptions({
    queryKey: queryKeys.permission(realmId, permissionId),
    queryFn: async () =>
      handleApiResponse(
        await getPermission({ path: { realmId, permissionDefinitionId: permissionId } })
      ),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== Roles ====================

export const rolesQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.roles(realmId),
    queryFn: async () => handleApiResponse(await listRoles({ path: { realmId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const roleQueryOptions = (realmId: string, roleId: string) =>
  queryOptions({
    queryKey: queryKeys.role(realmId, roleId),
    queryFn: async () => handleApiResponse(await getRole({ path: { realmId, roleId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const rolePermissionsQueryOptions = (realmId: string, roleId: string) =>
  queryOptions({
    queryKey: queryKeys.rolePermissions(realmId, roleId),
    queryFn: async () => handleApiResponse(await getRolePermissions({ path: { realmId, roleId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== User Roles ====================

export const userRolesQueryOptions = () =>
  queryOptions({
    queryKey: queryKeys.userRoles(),
    queryFn: async () => handleApiResponse(await getUserRoles()),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const adminUserRolesQueryOptions = (realmId: string, userId: string) =>
  queryOptions({
    queryKey: queryKeys.adminUserRoles(realmId, userId),
    queryFn: async () =>
      adminGetUserRoles({
        path: { realmId, userId },
      }),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

// ==================== Client Apps ====================

export const clientAppsQueryOptions = (
  realmId: string,
  filters: {
    page?: number
    pageSize?: number
  }
) =>
  queryOptions({
    queryKey: queryKeys.clientApps(realmId, filters),
    queryFn: async () =>
      handleApiResponse(
        await listClientApps({
          path: { realmId },
          query: {
            page: filters.page ?? 0,
            pageSize: filters.pageSize ?? 20,
          },
        })
      ),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const clientAppQueryOptions = (realmId: string, id: string) =>
  queryOptions({
    queryKey: queryKeys.clientApp(realmId, id),
    queryFn: async () =>
      handleApiResponse(await getClientApp({ path: { realmId, clientAppId: id } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== OAuth Configurations ====================

export const providerConfigsQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.oauthConfigs(realmId),
    queryFn: async () => {
      const response = await listOauthConfigs({ path: { realmId } })
      if (response.error) throw response.error
      return response.data as OAuthConfigResponse[]
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
    gcTime: GC_TIME_5_MIN,
  })

export const providerConfigQueryOptions = (realmId: string, providerType: string) =>
  queryOptions({
    queryKey: queryKeys.oauthConfig(realmId, providerType),
    queryFn: async () => {
      const response = await getOauthConfig({ path: { realmId, providerType } })
      if (response.error) throw response.error
      return response.data as OAuthConfigResponse
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
    gcTime: GC_TIME_5_MIN,
  })

// ==================== User Profile ====================

export const profileQueryOptions = queryOptions({
  queryKey: queryKeys.profile(),
  queryFn: async () => {
    const response = await getProfile()
    if (response.error) throw response.error
    return response.data
  },
  retry: RETRY_COUNT,
  staleTime: STALE_TIME_5_MIN,
  gcTime: GC_TIME_10_MIN,
})

export const turnstileStatusQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.turnstileStatus(realmId),
    queryFn: async () => {
      const response = await getTurnstileStatus({ path: { realmId } })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

// ==================== TOTP Status ====================

export const totpStatusQueryOptions = queryOptions({
  queryKey: queryKeys.totpStatus(),
  queryFn: async () => {
    const response = await handleGetTotpStatus()
    if (response.error) throw response.error
    return response.data
  },
  retry: RETRY_COUNT,
  staleTime: STALE_TIME_2_MIN,
  gcTime: GC_TIME_5_MIN,
})

// ==================== Subscriptions ====================

export const subscriptionQueryOptions = (realmId: string, clientAppId: string) =>
  queryOptions({
    queryKey: queryKeys.subscription(realmId, clientAppId),
    queryFn: async () => {
      const response = await getSubscriptionForClientApp({ path: { realmId, clientAppId } })
      if (response.error) {
        if (response.error.code === 404) return null
        throw response.error
      }
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const subscriptionDetailsQueryOptions = <TData>(
  realmId: string,
  subscriptionId: string,
  getCurrentState: () => TData | undefined
) =>
  queryOptions({
    queryKey: queryKeys.subscriptionDetails(realmId, subscriptionId),
    queryFn: async () => getCurrentState(),
    staleTime: STALE_TIME_2_MIN,
  })

export const userSubscriptionsQueryOptions = <TData>(
  realmId: string,
  clientAppIds: string,
  queryFn: () => Promise<TData>
) =>
  queryOptions({
    queryKey: queryKeys.userSubscriptions(realmId, clientAppIds),
    queryFn,
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
    gcTime: GC_TIME_5_MIN,
  })

// ==================== Subscription History ====================
// TODO: These will be auto-generated once backend OpenAPI includes subscription history endpoints
// For now, implement using direct fetch with proper error handling

/**
 * Unwraps a fetch Response object, throwing an error if the response is not OK.
 * This is used for direct fetch calls (not generated API client).
 */
async function unwrapFetchResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw new Error(`API request failed: ${response.status} ${response.statusText}`)
  }
  return response.json()
}

export async function getSubscriptionHistory(
  realmId: string,
  subscriptionId: string
): Promise<SingleSubscriptionHistoryResponse> {
  const response = await fetch(`/api/bill/${realmId}/subscriptions/${subscriptionId}/history`)
  return unwrapFetchResponse<SingleSubscriptionHistoryResponse>(response)
}

export async function getGlobalSubscriptionHistory(
  realmId: string,
  filters: HistoryFilters,
  page: number = 1,
  pageSize: number = 20
): Promise<GlobalSubscriptionHistoryResponse> {
  const params = new URLSearchParams()

  Object.entries(filters).forEach(([key, value]) => {
    if (value) {
      params.append(key, value)
    }
  })

  params.append('page', page.toString())
  params.append('pageSize', pageSize.toString())

  const response = await fetch(`/api/bill/${realmId}/subscriptions/history?${params.toString()}`)
  return unwrapFetchResponse<GlobalSubscriptionHistoryResponse>(response)
}

export const subscriptionHistoryQueryOptions = (realmId: string, subscriptionId: string) =>
  queryOptions({
    queryKey: queryKeys.subscriptionHistory(realmId, subscriptionId),
    queryFn: () => getSubscriptionHistory(realmId, subscriptionId),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
    gcTime: GC_TIME_5_MIN,
  })

export const globalSubscriptionHistoryQueryOptions = (
  realmId: string,
  filters: HistoryFilters,
  page: number = 1,
  pageSize: number = 20
) =>
  queryOptions({
    queryKey: queryKeys.globalSubscriptionHistory(realmId, filters, page, pageSize),
    queryFn: () => getGlobalSubscriptionHistory(realmId, filters, page, pageSize),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
    gcTime: GC_TIME_5_MIN,
  })

// ==================== Points ====================

export const pointsWalletQueryOptions = (realmId: string, userId: string) =>
  queryOptions({
    queryKey: queryKeys.pointsWallet(realmId, userId),
    queryFn: async () =>
      handleApiResponse(await getWallet({ path: { realmId, userId } })) as PointsWalletResponse,
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const pointsTransactionsQueryOptions = (
  realmId: string,
  filters: {
    userId?: string
    clientAppId?: string
    subscriptionId?: string
    transactionType?: string
    bucketId?: string
    startTime?: string
    endTime?: string
    page?: number
    pageSize?: number
  }
) =>
  queryOptions({
    queryKey: queryKeys.pointsTransactions(realmId, filters),
    queryFn: async () => {
      const data = handleApiResponse(
        await listTransactions({
          path: { realmId },
          query: {
            userId: filters.userId,
            clientAppId: filters.clientAppId,
            subscriptionId: filters.subscriptionId,
            transactionType: filters.transactionType,
            bucketId: filters.bucketId,
            startTime: filters.startTime,
            endTime: filters.endTime,
            page: filters.page,
            pageSize: filters.pageSize,
          },
        })
      )
      return {
        total: data.total,
        page: data.page,
        pageSize: data.pageSize,
        transactions: data.items,
      }
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

// ==================== Points Default Config ====================

export const pointsDefaultConfigQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.pointsDefaultConfig(realmId),
    queryFn: async () => {
      try {
        const response = await getRealmDefaultConfig({ path: { realmId } })
        if (response.error) handleApiErrorWithStatus(response.error)
        return response.data
      } catch (error) {
        handleApiErrorWithStatus(error)
      }
    },
    retry: clientErrorRetry,
    staleTime: STALE_TIME_5_MIN,
  })

// Wrapper function for mutation use
export const updatePointsDefaultConfigMutation = async (
  realmId: string,
  data: {
    registrationBonusPoints: number
    freePeriodicPointsAmount: number
    freePeriodicGrantPeriodType: 'once' | 'daily' | 'weekly' | 'monthly'
    freePeriodicValidityDays: number
  }
) => {
  try {
    const response = await updateRealmDefaultConfig({
      path: { realmId },
      body: data,
    })
    if (response.error) handleApiErrorWithStatus(response.error)
    return response.data
  } catch (error) {
    handleApiErrorWithStatus(error)
  }
}

// ==================== One-Time Mappings ====================

export const oneTimeMappingsQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.oneTimeMappings(realmId),
    queryFn: async () => {
      const response = await listOneTimeMappings({
        path: { realmId },
      })
      if (response.error) throw response.error
      return (response.data as OneTimeMappingExtResponse).items ?? []
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== Purchase History ====================

export interface PurchaseHistoryFilters {
  page?: number
  pageSize?: number
  paymentProvider?: string
  startDate?: string
  endDate?: string
}

export const purchaseHistoryQueryOptions = (
  realmId: string,
  filters: PurchaseHistoryFilters = {}
) =>
  queryOptions({
    queryKey: queryKeys.purchaseHistory(realmId, filters as Record<string, unknown>),
    queryFn: async () => {
      const query: Record<string, unknown> = {}
      if (filters.page !== undefined) query.page = filters.page
      if (filters.pageSize !== undefined) query.page_size = filters.pageSize
      if (filters.paymentProvider !== undefined) query.payment_provider = filters.paymentProvider
      if (filters.startDate !== undefined) query.start_date = filters.startDate
      if (filters.endDate !== undefined) query.end_date = filters.endDate

      const response = await getPurchaseHistory({
        path: { realmId },
        query: query as {
          page?: number | null
          page_size?: number | null
          payment_provider?: string | null
          start_date?: string | null
          end_date?: string | null
        },
      })
      if (response.error) throw response.error
      return response.data as PurchaseHistoryResponse
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const paymentAttemptStatusQueryOptions = (realmId: string, attemptId: string) =>
  queryOptions({
    queryKey: queryKeys.paymentAttemptStatus(realmId, attemptId),
    queryFn: async () => {
      if (!attemptId) {
        throw new Error('attemptId is required')
      }
      const response = await getPaymentAttemptStatus({
        path: { realmId, attemptId },
      })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: TIME_CONSTANTS.ONE_MINUTE, // More frequent updates for payment status
    refetchInterval: (query) => {
      // Handle test environment where query might be undefined or a mock
      if (!query || !query.state) {
        return false
      }
      // Poll more frequently for pending payments
      const status = query.state.data as PaymentAttemptStatusResponse | undefined
      if (status && (status.status === 'Pending' || status.status === 'RequiresAction')) {
        return TIME_CONSTANTS.ONE_MINUTE
      }
      return false // Stop polling for completed/failed payments
    },
  })

export const paymentProvidersQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.paymentProviders(realmId),
    queryFn: async () => {
      const response = await listPaymentProviders({
        path: { realmId },
      })
      if (response.error) throw response.error
      return response.data?.providers ?? []
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== Audit ====================

export const auditListQueryOptions = (
  realmId: string,
  filters: {
    page?: number
    pageSize?: number
    category?: string
    action?: string
    actorId?: string
    startTime?: string
    endTime?: string
  }
) =>
  queryOptions({
    queryKey: queryKeys.audit(realmId, filters),
    queryFn: async () =>
      handleApiResponse(
        await listAuditEvents({
          path: { realmId },
          query: {
            page: filters.page ?? 0,
            pageSize: filters.pageSize ?? 20,
            category: filters.category,
            action: filters.action,
            actorId: filters.actorId,
            startTime: filters.startTime,
            endTime: filters.endTime,
          },
        })
      ),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const auditDetailQueryOptions = (realmId: string, eventId: string) =>
  queryOptions({
    queryKey: queryKeys.auditDetail(realmId, eventId),
    queryFn: async () => handleApiResponse(await getAuditEvent({ path: { realmId, eventId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== Dashboard ====================

export const dashboardStatsQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.dashboardStats(realmId),
    queryFn: async () => {
      const response = await getDashboardStats({ path: { realmId } })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

// ==================== Email Status ====================

export const emailStatusQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.emailStatus(realmId),
    queryFn: async () => {
      const response = await emailStatus({ path: { realmId } })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

// ==================== API Keys ====================

export const apiKeysQueryOptions = (
  realmId: string,
  filters: {
    page?: number
    pageSize?: number
  }
) =>
  queryOptions({
    queryKey: queryKeys.apiKeys(realmId, filters),
    queryFn: async () =>
      handleApiResponse(
        await listApiKeys({
          path: { realmId },
          query: {
            page: filters.page ?? 0,
            pageSize: filters.pageSize ?? 20,
          },
        })
      ),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const apiKeyQueryOptions = (realmId: string, id: string) =>
  queryOptions({
    queryKey: queryKeys.apiKey(realmId, id),
    queryFn: async () => handleApiResponse(await getApiKey({ path: { realmId, apiKeyId: id } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== API Key Roles ====================

export const adminApiKeyRolesQueryOptions = (realmId: string, apiKeyId: string) =>
  queryOptions({
    queryKey: queryKeys.apiKeyRoles(realmId, apiKeyId),
    queryFn: async () =>
      handleApiResponse(
        await adminGetApiKeyRoles({
          path: { realmId, apiKeyId },
        })
      ),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const updateApiKeyRolesMutation = async (
  realmId: string,
  apiKeyId: string,
  roleIds: string[]
) => {
  try {
    const response = await adminUpdateApiKeyRoles({
      path: { realmId, apiKeyId },
      body: { roleIds },
    })
    if (response.error) handleApiErrorWithStatus(response.error)
    return response.data
  } catch (error) {
    handleApiErrorWithStatus(error)
  }
}

// ==================== Entitlement Mappings ====================

export interface EntitlementMappingFilters {
  paymentProvider?: string
  enabled?: boolean
  page?: number
  pageSize?: number
}

export const entitlementMappingsQueryOptions = (
  realmId: string,
  filters: EntitlementMappingFilters = {}
) =>
  queryOptions({
    queryKey: queryKeys.entitlementMappings(realmId, filters as Record<string, unknown>),
    queryFn: async () => {
      const query: Record<string, unknown> = {}
      if (filters.paymentProvider !== undefined) query.paymentProvider = filters.paymentProvider
      if (filters.enabled !== undefined) query.enabled = filters.enabled
      if (filters.page !== undefined) query.page = filters.page
      if (filters.pageSize !== undefined) query.pageSize = filters.pageSize

      const response = await client.get<EntitlementMappingListResponse>({
        url: `/api/bill/${realmId}/entitlement-mappings`,
        query,
      })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const entitlementMappingQueryOptions = (realmId: string, mappingId: string) =>
  queryOptions({
    queryKey: queryKeys.entitlementMapping(realmId, mappingId),
    queryFn: async () => {
      const response = await getEntitlementMapping({
        path: { realmId, mappingId },
      })
      if (response.error) throw response.error
      return response.data as EntitlementMappingResponse
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

// ==================== Admin Subscriptions ====================

export interface SubscriptionFilters {
  entitlementKey?: string
  status?: string
  paymentProvider?: string
  page?: number
  pageSize?: number
}

export const subscriptionsQueryOptions = (realmId: string, filters: SubscriptionFilters = {}) =>
  queryOptions({
    queryKey: queryKeys.subscriptions(realmId, filters as Record<string, unknown>),
    queryFn: async () => {
      const query: Record<string, unknown> = {}
      if (filters.entitlementKey !== undefined) query.entitlementKey = filters.entitlementKey
      if (filters.status !== undefined) query.status = filters.status
      if (filters.paymentProvider !== undefined) query.paymentProvider = filters.paymentProvider
      if (filters.page !== undefined) query.page = filters.page
      if (filters.pageSize !== undefined) query.pageSize = filters.pageSize

      const response = await client.get<SubscriptionListResponse>({
        url: `/api/bill/${realmId}/subscriptions`,
        query,
      })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const subscriptionDetailQueryOptions = (realmId: string, subscriptionId: string) =>
  queryOptions({
    queryKey: queryKeys.adminSubscription(realmId, subscriptionId),
    queryFn: async () => {
      const response = await getSubscription({
        path: { realmId, subscriptionId },
      })
      if (response.error) throw response.error
      return response.data as SubscriptionDetailResponse
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

// ==================== Credit Buckets ====================

export const creditBucketsListQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.creditBucketsList(realmId),
    queryFn: async () =>
      handleApiResponse(await listCreditBucketsHandler({ path: { realmId } })) as BucketResponse[],
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const creditBucketDetailQueryOptions = (realmId: string, bucketId: string) =>
  queryOptions({
    queryKey: queryKeys.creditBucket(realmId, bucketId),
    queryFn: async () =>
      handleApiResponse(
        await getCreditBucketHandler({ path: { realmId, bucketId } })
      ) as BucketDetailResponse,
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const creditBucketOverviewQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.creditBucketOverview(realmId),
    queryFn: async () =>
      handleApiResponse(
        await getBucketOverviewHandler({ path: { realmId } })
      ) as BucketOverviewResponse,
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

/**
 * Wallets grouped by (bucket_id, user_id) for a realm — `GET /api/points/{realmId}/wallets`
 * via the generated `listWallets` SDK (returns `ListWalletsByBucketResponse`).
 *
 * Backend scoping (Gap #2 fix): the endpoint is `points.view`-gated, and the service
 * hard-scopes the result to the caller's identity.
 *   - `points.view`-only callers receive ONLY their own wallet rows (server-injected
 *     `user_id`; the client cannot target another user — `search` is stripped
 *     server-side for non-managers).
 *   - `points.manage` holders receive the full realm-wide (cross-user) set.
 *   - FE-D05 (user points page) still client-filters `items` by the current `userId`
 *     via `deriveUserPointsView` — now a defensive no-op for view-only callers, kept
 *     because it is harmless and still correct.
 *   - FE-D10 (admin wallets) consumes the full `items` + `crossBucketTotal`.
 *
 * For a `points.view`-only caller `crossBucketTotal` is that user's own cross-bucket
 * total; for a `points.manage` caller it is the realm-wide cross-user total.
 */
export const walletsByBucketQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.walletsByBucket(realmId),
    queryFn: async () =>
      handleApiResponse(await listWallets({ path: { realmId } })) as ListWalletsByBucketResponse,
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })
