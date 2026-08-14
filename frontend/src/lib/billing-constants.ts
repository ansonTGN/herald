export const PAYMENT_PROVIDERS = {
  CREEM: 'creem',
  STRIPE: 'stripe',
  APPLE: 'apple',
  GOOGLE: 'google',
  WECHAT: 'wechat',
} as const

export const STRIPE_CONFIG_KEYS = {
  API_KEY: 'api_key',
  PUBLISHABLE_KEY: 'publishable_key',
  WEBHOOK_SECRET: 'webhook_secret',
  TIMEOUT: 'timeout',
  ASYNC_POINTS_STRATEGY: 'async_points_strategy',
} as const

export const APPLE_CONFIG_KEYS = {
  BUNDLE_ID: 'bundle_id',
  ISSUER_ID: 'issuer_id',
  KEY_ID: 'key_id',
  PRIVATE_KEY_P8: 'private_key_p8',
  ENVIRONMENT: 'environment',
} as const

export const GOOGLE_CONFIG_KEYS = {
  PACKAGE_NAME: 'package_name',
  SERVICE_ACCOUNT_JSON: 'service_account_json',
} as const

export type PaymentProvider = (typeof PAYMENT_PROVIDERS)[keyof typeof PAYMENT_PROVIDERS]

export const STRIPE_ERROR_CODES = {
  INVALID_API_KEY: 'INVALID_API_KEY',
  WEBHOOK_NOT_CONFIGURED: 'WEBHOOK_NOT_CONFIGURED',
  UNAUTHORIZED: 'UNAUTHORIZED',
  FORBIDDEN: 'FORBIDDEN',
  VALIDATION_ERROR: 'VALIDATION_ERROR',
  SERVICE_UNAVAILABLE: 'SERVICE_UNAVAILABLE',
  INVALID_PUBLIC_KEY_FORMAT: 'INVALID_PUBLIC_KEY_FORMAT',
  INVALID_SECRET_KEY_FORMAT: 'INVALID_SECRET_KEY_FORMAT',
  INVALID_WEBHOOK_SECRET_FORMAT: 'INVALID_WEBHOOK_SECRET_FORMAT',
  NETWORK_ERROR: 'NETWORK_ERROR',
} as const

export const STRIPE_KEY_PREFIXES = {
  PUBLISHABLE: 'pk_',
  SECRET: 'sk_',
  WEBHOOK: 'whsec_',
} as const

export const BILLING_PERIODS = {
  MONTHLY: 'monthly',
  YEARLY: 'yearly',
} as const

export type BillingPeriod = (typeof BILLING_PERIODS)[keyof typeof BILLING_PERIODS]

export const PLAN_TIERS = {
  FREE: 'free',
  PRO: 'pro',
  ENTERPRISE: 'enterprise',
} as const

export type PlanTier = (typeof PLAN_TIERS)[keyof typeof PLAN_TIERS]
