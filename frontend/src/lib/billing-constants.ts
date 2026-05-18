/**
 * Billing and payment provider constants
 * Provides type-safe constants for payment providers and error codes
 */

/**
 * Payment provider types
 */
export const PAYMENT_PROVIDERS = {
  CREEM: 'creem',
  STRIPE: 'stripe',
} as const

export const STRIPE_CONFIG_KEYS = {
  ENABLED: 'enabled',
  API_KEY: 'api_key',
  PUBLISHABLE_KEY: 'publishable_key',
  WEBHOOK_SECRET: 'webhook_secret',
  TIMEOUT: 'timeout',
} as const

export type PaymentProvider = (typeof PAYMENT_PROVIDERS)[keyof typeof PAYMENT_PROVIDERS]

/**
 * Stripe-specific error codes
 */
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

/**
 * Stripe API key prefixes
 */
export const STRIPE_KEY_PREFIXES = {
  PUBLISHABLE: 'pk_',
  SECRET: 'sk_',
  WEBHOOK: 'whsec_',
} as const

/**
 * Billing period types
 */
export const BILLING_PERIODS = {
  MONTHLY: 'monthly',
  YEARLY: 'yearly',
} as const

export type BillingPeriod = (typeof BILLING_PERIODS)[keyof typeof BILLING_PERIODS]

/**
 * Plan tier types
 */
export const PLAN_TIERS = {
  FREE: 'free',
  PRO: 'pro',
  ENTERPRISE: 'enterprise',
} as const

export type PlanTier = (typeof PLAN_TIERS)[keyof typeof PLAN_TIERS]
