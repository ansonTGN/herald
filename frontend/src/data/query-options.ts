import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { redirect } from '@tanstack/react-router'
import {
  listUsers,
  getUser,
  listPermissions,
  getPermission,
  listRoles,
  getRole,
  getRolePermissions,
  getUserRoles,
  adminGetUserRoles,
  listRealmsPaginated,
  getRealm,
  listClientApps,
  getClientApp,
  listOauthConfigs,
  getOauthConfig,
  getPublicConfig,
  getTurnstileStatus,
  getProfile,
  handleGetTotpStatus,
  listPlans,
  getPlan,
  listProducts,
  getProduct,
  getProductPlans,
  getSubscriptionForClientApp,
  listPlanAssignments,
  listPlanAssignmentsBatch,
  listAccounts,
  getAccount,
  listTransactions,
  listPlanConfigs,
  getRealmDefaultConfig,
  updateRealmDefaultConfig,
  getFreeUserStatistics,
  listPlanPaymentProviders,
  listPointsPackages,
  getPointsPackage,
  getPointsPackagePurchaseHistory,
  getPointsPackagePurchaseDetails,
  getPaymentAttemptStatus,
  listPaymentProviders,
  listPaymentProviderMappings,
  listAuditEvents,
  getAuditEvent,
  getDashboardStats,
  emailStatus,
} from '@/lib/api-generated'
import { handleApiResponse } from '@/lib/api-utils'
import type {
  OAuthConfigResponse,
  SubscriptionPlanResponse,
  PaymentAttemptStatusResponse,
  PointsAccountResponse,
} from '@/lib/api-generated'
import type {
  HistoryFilters,
  SingleSubscriptionHistoryResponse,
  GlobalSubscriptionHistoryResponse,
} from '@/types/billing'
import { TIME_CONSTANTS, QUERY_KEYS } from '@/lib/constants'

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
  billingPlans: (realmId: string, filters?: { page?: number; pageSize?: number }) =>
    [QUERY_KEYS.BILLING_PLANS, realmId, filters] as const,
  billingPlan: (realmId: string, planId: string) =>
    [QUERY_KEYS.BILLING_PLAN, realmId, planId] as const,
  billingProducts: (realmId: string) => [QUERY_KEYS.BILLING_PRODUCTS, realmId] as const,
  billingProduct: (realmId: string, productId: string) =>
    [QUERY_KEYS.BILLING_PRODUCT, realmId, productId] as const,
  billingProductPlans: (realmId: string, productId: string) =>
    [QUERY_KEYS.BILLING_PRODUCT_PLANS, realmId, productId] as const,
  planProviders: (realmId: string, planId: string) =>
    [QUERY_KEYS.PLAN_PROVIDERS, realmId, planId] as const,
  subscription: (realmId: string, clientAppId: string) =>
    [QUERY_KEYS.SUBSCRIPTION, realmId, clientAppId] as const,
  subscriptionDetails: (realmId: string, subscriptionId: string) =>
    [QUERY_KEYS.SUBSCRIPTION_DETAILS, realmId, subscriptionId] as const,
  planAssignments: (realmId: string, clientAppId: string) =>
    [QUERY_KEYS.PLAN_ASSIGNMENTS, realmId, clientAppId] as const,
  planAssignmentsList: (realmId: string) => [QUERY_KEYS.PLAN_ASSIGNMENTS, realmId] as const,
  planAssignmentsBatch: (realmId: string, clientAppIds: string[]) =>
    [QUERY_KEYS.PLAN_ASSIGNMENTS, realmId, 'batch', clientAppIds] as const,
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
  pointsAccounts: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.POINTS_ACCOUNTS, realmId, filters] as const,
  pointsAccount: (realmId: string, userId: string) =>
    [QUERY_KEYS.POINTS_ACCOUNT, realmId, userId] as const,
  pointsTransactions: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.POINTS_TRANSACTIONS, realmId, filters] as const,
  pointsPlanConfigs: (realmId: string) => [QUERY_KEYS.POINTS_PLAN_CONFIGS, realmId] as const,
  realmConfig: (realmId: string) => [QUERY_KEYS.REALM_CONFIG, realmId] as const,
  emailStatus: (realmId: string) => [QUERY_KEYS.EMAIL_STATUS, realmId] as const,
  freeUserStats: (realmId: string, dateRange?: { startDate?: string; endDate?: string }) =>
    [QUERY_KEYS.FREE_USER_STATS, realmId, dateRange] as const,
  userRoles: () => [QUERY_KEYS.USER_ROLES] as const,
  pointsPackages: (realmId: string) => [QUERY_KEYS.POINTS_PACKAGES, realmId] as const,
  pointsPackage: (realmId: string, packageId: string) =>
    [QUERY_KEYS.POINTS_PACKAGE, realmId, packageId] as const,
  pointsPackagePurchases: (realmId: string, filters: Record<string, unknown>) =>
    [QUERY_KEYS.POINTS_PACKAGE_PURCHASES, realmId, filters] as const,
  paymentAttemptStatus: (realmId: string, attemptId: string) =>
    [QUERY_KEYS.PAYMENT_ATTEMPT_STATUS, realmId, attemptId] as const,
  audit: (realmId: string, filters?: Record<string, unknown>) =>
    [QUERY_KEYS.AUDIT_EVENTS, realmId, filters ?? {}] as const,
  auditDetail: (realmId: string, eventId: string) =>
    [QUERY_KEYS.AUDIT_EVENT, realmId, eventId] as const,
  dashboardStats: (realmId: string) => [QUERY_KEYS.DASHBOARD_STATS, realmId] as const,
  featureAvailability: (realmId: string) => [QUERY_KEYS.FEATURE_AVAILABILITY, realmId] as const,
}

function extractNestedArray<T>(response: unknown, key: string): T[] {
  if (!response || typeof response !== 'object') {
    return []
  }
  const value = (response as Record<string, unknown>)[key]
  return Array.isArray(value) ? (value as T[]) : []
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
    queryFn: async () => handleApiResponse(await getRealm({ path: { realmId } })),
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export interface FeatureAvailabilityResponse {
  admin: {
    billingVisible: boolean
    billingConfigVisible: boolean
    productsVisible: boolean
    plansVisible: boolean
    invoicesVisible: boolean
    subscriptionHistoryVisible: boolean
    pointsVisible: boolean
    pointsPackagesVisible: boolean
  }
  user: {
    pointsVisible: boolean
    pointsPurchaseVisible: boolean
    subscriptionVisible: boolean
    invoicesVisible: boolean
  }
  facts: {
    hasPaymentProviders: boolean
    hasProducts: boolean
    hasPlans: boolean
    hasPlanPaymentMappings: boolean
    hasPointsPackages: boolean
    hasPointsPackagePaymentMappings: boolean
    hasInvoiceSellerConfig: boolean
    hasInvoices: boolean
    hasSubscriptionHistory: boolean
  }
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
  }
) =>
  queryOptions({
    queryKey: queryKeys.users(realmId, filters),
    queryFn: async () =>
      handleApiResponse(
        await listUsers({
          path: { realmId },
          query: {
            page: filters.page ?? 0,
            pageSize: filters.pageSize ?? 20,
            email: filters.email,
          },
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

// ==================== Subscription Plans ====================

export const subscriptionPlansQueryOptions = (
  realmId: string,
  filters?: {
    page?: number
    pageSize?: number
  }
) =>
  queryOptions({
    queryKey: queryKeys.billingPlans(realmId, filters),
    queryFn: async () => {
      const response = await listPlans({ path: { realmId } })
      if (response.error) throw response.error
      const allPlans = extractNestedArray<SubscriptionPlanResponse>(response.data, 'plans')

      // Client-side pagination (backend doesn't support pagination for plans)
      const page = filters?.page ?? 0
      const pageSize = filters?.pageSize ?? 20
      const total = allPlans.length
      const startIndex = page * pageSize
      const endIndex = startIndex + pageSize
      const items = allPlans.slice(startIndex, endIndex)

      return {
        page,
        pageSize,
        total,
        items,
      }
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const subscriptionPlanQueryOptions = (realmId: string, planId: string) =>
  queryOptions({
    queryKey: queryKeys.billingPlan(realmId, planId),
    queryFn: async () => {
      const response = await getPlan({ path: { realmId, planId } })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const subscriptionPlanProvidersQueryOptions = (realmId: string, planId: string) =>
  queryOptions({
    queryKey: queryKeys.planProviders(realmId, planId),
    queryFn: async () => {
      const response = await listPlanPaymentProviders({
        path: { realmId, planId },
      })
      if (response.error) throw new Error(response.error.message)
      return response.data ?? []
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

// ==================== Products ====================

export const productsQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.billingProducts(realmId),
    queryFn: async () => {
      const response = await listProducts({
        path: { realmId },
      })
      if (response.error) throw response.error
      return extractNestedArray<{
        id: string
        code: string
        title: string
        description?: string | null
        enabled: boolean
        plansCount: number
        realmId: string
        createdAt: string
        updatedAt: string
      }>(response.data, 'products')
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const productQueryOptions = (realmId: string, productId: string) =>
  queryOptions({
    queryKey: queryKeys.billingProduct(realmId, productId),
    queryFn: async () => {
      const response = await getProduct({ path: { realmId, productId } })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const productPlansQueryOptions = (realmId: string, productId: string) =>
  queryOptions({
    queryKey: queryKeys.billingProductPlans(realmId, productId),
    queryFn: async () => {
      const response = await getProductPlans({ path: { realmId, productId } })
      if (response.error) throw response.error
      return extractNestedArray<{
        id: string
        name: string
        title: string
        active: boolean
        price: number
        currency: string
        type: string
      }>(response.data, 'plans')
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
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

// ==================== Subscription Plan Assignments ====================

export const subscriptionPlanAssignmentsQueryOptions = (realmId: string, clientAppId: string) =>
  queryOptions({
    queryKey: queryKeys.planAssignments(realmId, clientAppId),
    queryFn: async () => {
      const response = await listPlanAssignments({ path: { realmId, clientAppId } })
      if (response.error) throw response.error
      return response.data?.assignments || []
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const subscriptionPlanAssignmentsBatchQueryOptions = (
  realmId: string,
  clientAppIds: string[]
) =>
  queryOptions({
    queryKey: queryKeys.planAssignmentsBatch(realmId, clientAppIds),
    queryFn: async () => {
      const response = await listPlanAssignmentsBatch({
        path: { realmId },
        query: { clientAppIds: clientAppIds.join(',') },
      })
      if (response.error) throw response.error
      return response.data?.assignments || []
    },
    enabled: clientAppIds.length > 0,
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
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

interface PointsAccountsListResponse {
  total: number
  page: number
  pageSize: number
  items: Array<{
    id: string
    userId: string
    realmId: string
    balance: number
    totalRecharged: number
    totalConsumed: number
    status: string
    createdAt: string
    updatedAt: string
    currency: string
    userName?: string | null
    userEmail?: string | null
  }>
}

interface PointsTransactionsListResponse {
  total: number
  page: number
  pageSize: number
  items: Array<{
    id: string
    accountId: string
    userId: string
    realmId: string
    transactionType: string
    amount: number
    balanceAfter: number
    description: string | null
    clientAppId: string | null
    subscriptionId: string | null
    externalRefId: string | null
    createdAt: string
  }>
}

export const pointsAccountsQueryOptions = (
  realmId: string,
  filters: {
    page?: number
    pageSize?: number
    search?: string
    status?: string
  }
) =>
  queryOptions({
    queryKey: queryKeys.pointsAccounts(realmId, filters),
    queryFn: async () => {
      const response = await listAccounts({
        path: { realmId },
        query: {
          page: filters.page,
          pageSize: filters.pageSize,
          search: filters.search,
          status: filters.status,
        },
      })
      if (response.error) throw response.error
      const data = response.data as unknown
      if (!data || typeof data !== 'object') {
        throw new Error('Invalid response data')
      }
      const paginated = data as PointsAccountsListResponse & {
        data?: PointsAccountsListResponse['items']
      }
      const accounts = paginated.items ?? paginated.data ?? []
      return {
        total: paginated.total,
        page: paginated.page,
        pageSize: paginated.pageSize,
        accounts: accounts.map((account) => ({
          id: account.id,
          userId: account.userId,
          userName: account.userName ?? undefined,
          userEmail: account.userEmail ?? undefined,
          realmId: account.realmId,
          balance: account.balance,
          totalRecharged: account.totalRecharged,
          totalConsumed: account.totalConsumed,
          status: account.status,
          createdAt: account.createdAt,
          updatedAt: account.updatedAt,
          unit: account.currency,
        })),
      }
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const pointsAccountQueryOptions = (realmId: string, userId: string) =>
  queryOptions({
    queryKey: queryKeys.pointsAccount(realmId, userId),
    queryFn: async () => {
      const response = await getAccount({ path: { realmId, userId } })
      if (response.error) throw response.error
      const data = response.data as unknown
      if (!data || typeof data !== 'object') {
        return null
      }
      return data as PointsAccountResponse | null
    },
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
    startTime?: string
    endTime?: string
    page?: number
    pageSize?: number
  }
) =>
  queryOptions({
    queryKey: queryKeys.pointsTransactions(realmId, filters),
    queryFn: async () => {
      const response = await listTransactions({
        path: { realmId },
        query: {
          userId: filters.userId,
          clientAppId: filters.clientAppId,
          subscriptionId: filters.subscriptionId,
          transactionType: filters.transactionType,
          startTime: filters.startTime,
          endTime: filters.endTime,
          page: filters.page,
          pageSize: filters.pageSize,
        },
      })
      if (response.error) throw response.error
      const data = response.data as unknown
      if (!data || typeof data !== 'object') {
        throw new Error('Invalid response data')
      }
      const paginated = data as PointsTransactionsListResponse
      return {
        total: paginated.total,
        page: paginated.page,
        pageSize: paginated.pageSize,
        transactions: paginated.items,
      }
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const pointsPlanConfigsQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.pointsPlanConfigs(realmId),
    queryFn: async () => {
      const response = await listPlanConfigs({ path: { realmId } })
      if (response.error) throw response.error
      return extractNestedArray<{
        configId: string
        realmId: string
        planId: string
        pointsPerPeriod: number
        grantOnSubscribe: boolean
        grantPeriodType: string
        maxPeriods: number | null
        validityDays: number
        active: boolean
        createdAt: string
        updatedAt: string
      }>(response.data, 'configs')
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

// ==================== Realm Default Config ====================

export const realmConfigQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.realmConfig(realmId),
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
export const updateRealmDefaultConfigMutation = async (
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

// ==================== Free User Statistics ====================

export const freeUserStatsQueryOptions = (
  realmId: string,
  dateRange?: { startDate?: string; endDate?: string }
) =>
  queryOptions({
    queryKey: queryKeys.freeUserStats(realmId, dateRange),
    queryFn: async () => {
      try {
        const response = await getFreeUserStatistics({
          path: { realmId },
          query: dateRange,
        })
        if (response.error) handleApiErrorWithStatus(response.error)
        return response.data
      } catch (error) {
        handleApiErrorWithStatus(error)
      }
    },
    retry: clientErrorRetry,
    staleTime: STALE_TIME_2_MIN,
    refetchInterval: TIME_CONSTANTS.FIVE_MINUTES,
  })

// ==================== Points Packages ====================

export const pointsPackagesQueryOptions = (realmId: string) =>
  queryOptions({
    queryKey: queryKeys.pointsPackages(realmId),
    queryFn: async () => {
      const response = await listPointsPackages({
        path: { realmId },
      })
      if (response.error) throw response.error
      // Response has .packages property, not .items
      return response.data?.packages ?? []
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const pointsPackageQueryOptions = (realmId: string, packageId: string) =>
  queryOptions({
    queryKey: queryKeys.pointsPackage(realmId, packageId),
    queryFn: async () => {
      const response = await getPointsPackage({
        path: { realmId, packageId },
      })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
  })

export const pointsPackagePurchaseHistoryQueryOptions = (
  realmId: string,
  filters: {
    page?: number
    pageSize?: number
    userId?: string
    status?: string
    startTime?: string
    endTime?: string
  }
) =>
  queryOptions({
    queryKey: queryKeys.pointsPackagePurchases(realmId, filters),
    queryFn: async () => {
      const page = filters.page ?? 1
      const pageSize = filters.pageSize ?? 20
      const response = await getPointsPackagePurchaseHistory({
        path: { realmId },
        query: {
          offset: (page - 1) * pageSize,
          limit: pageSize,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_2_MIN,
  })

export const pointsPackagePurchaseDetailsQueryOptions = (realmId: string, purchaseId: string) =>
  queryOptions({
    queryKey: ['points-package-purchase-details', realmId, purchaseId] as const,
    queryFn: async () => {
      const response = await getPointsPackagePurchaseDetails({
        path: { realmId, purchaseId },
      })
      if (response.error) throw response.error
      return response.data
    },
    retry: RETRY_COUNT,
    staleTime: STALE_TIME_5_MIN,
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
    queryKey: ['payment-providers', realmId] as const,
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

export const paymentProviderMappingsQueryOptions = (realmId: string, packageId: string) =>
  queryOptions({
    queryKey: ['payment-provider-mappings', realmId, packageId] as const,
    queryFn: async () => {
      const response = await listPaymentProviderMappings({
        path: { realmId, packageId },
      })
      if (response.error) throw response.error
      return response.data?.mappings ?? []
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
