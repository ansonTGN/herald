/**
 * Shared constants used across the application
 */

/**
 * Default page size for paginated lists
 */
export const DEFAULT_PAGE_SIZE = 20

/**
 * Low balance threshold for points wallets
 * Wallets with balance below this value may trigger warnings
 */
export const LOW_POINTS_THRESHOLD = 100

/**
 * Time constants in milliseconds
 */
export const TIME_CONSTANTS = {
  ONE_MINUTE: 60 * 1000,
  FIVE_MINUTES: 5 * 60 * 1000,
  TWO_MINUTES: 2 * 60 * 1000,
} as const

/**
 * Query cache keys for React Query
 * These should be used consistently across the application
 */
export const QUERY_KEYS = {
  PUBLIC_CONFIG: 'public-config',
  REALMS: 'realms',
  REALM: 'realm',
  USERS: 'users',
  USER: 'user',
  PERMISSIONS: 'permissions',
  PERMISSION: 'permission',
  ROLES: 'roles',
  ROLE: 'role',
  ROLE_PERMISSIONS: 'role-permissions',
  ADMIN_USER_ROLES: 'admin-user-roles',
  USER_ROLES: 'user-roles',
  CLIENT_APPS: 'client-apps',
  CLIENT_APP: 'client-app',
  OAUTH_CONFIGS: 'oauth-configs',
  PROFILE: 'profile',
  TOTP_STATUS: 'totp-status',
  TURNSTILE_STATUS: 'turnstile-status',
  USER_SUBSCRIPTIONS: 'user-subscriptions',
  SUBSCRIPTION_DETAILS: 'subscription-details',
  BILLING_PLANS: 'subscription-plans',
  BILLING_PLAN: 'subscription-plan',
  BILLING_PRODUCTS: 'billing-products',
  BILLING_PRODUCT: 'billing-product',
  BILLING_PRODUCT_PLANS: 'billing-product-plans',
  PLAN_PROVIDERS: 'subscription-plan-providers',
  SUBSCRIPTION: 'subscription',
  PLAN_ASSIGNMENTS: 'subscription-plan-assignments',
  SUBSCRIPTION_HISTORY: 'subscription-history',
  GLOBAL_SUBSCRIPTION_HISTORY: 'global-subscription-history',
  POINTS_WALLETS: 'points-wallets',
  POINTS_WALLET: 'points-wallet',
  POINTS_TRANSACTIONS: 'points-transactions',
  POINTS_PLAN_CONFIGS: 'points-plan-configs',
  POINTS_PACKAGES: 'points-packages',
  POINTS_PACKAGE: 'points-package',
  POINTS_PACKAGE_PURCHASES: 'points-package-purchases',
  PAYMENT_ATTEMPT_STATUS: 'payment-attempt-status',
  REALM_CONFIG: 'realm-config',
  FREE_USER_STATS: 'free-user-stats',
  EMAIL_STATUS: 'email-status',
  AUDIT_EVENTS: 'audit-events',
  AUDIT_EVENT: 'audit-event',
  DASHBOARD_STATS: 'dashboard-stats',
  FEATURE_AVAILABILITY: 'feature-availability',
  API_KEYS: 'api-keys',
  API_KEY: 'api-key',
  API_KEY_ROLES: 'api-key-roles',
} as const

/**
 * Filter constant for "all" option in dropdowns
 */
export const FILTER_ALL_VALUE = '__all__'

/**
 * UTC time boundary suffixes
 */
export const UTC_TIME_BOUNDARIES = {
  START: 'T00:00:00.000Z',
  END: 'T23:59:59.999Z',
} as const
