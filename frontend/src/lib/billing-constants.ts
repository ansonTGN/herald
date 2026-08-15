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

// ---------------------------------------------------------------------------
// Provider-scoped mapping-form rules (single decision point — the create
// dialog and its schema consume these; do not re-test provider strings inline).
// ---------------------------------------------------------------------------

/** Providers whose external price id / price come from a hosted catalog that
 *  Herald syncs from the provider dashboard (Stripe / Creem). */
const CATALOG_SYNC_PROVIDERS: ReadonlySet<string> = new Set([
  PAYMENT_PROVIDERS.STRIPE,
  PAYMENT_PROVIDERS.CREEM,
])

/**
 * Whether the mapping form shows the "External Price ID" field. Catalog
 * providers take the id from their dashboard; IAP store ids and WeChat's
 * self-defined product ids have no price-id counterpart.
 */
export function providerShowsExternalPriceId(provider?: string | null): boolean {
  return !!provider && CATALOG_SYNC_PROVIDERS.has(provider)
}

/**
 * Whether the mapping price/currency are configured by hand in the form.
 * WeChat Pay v3 has no hosted product catalog (wechat-support PRD §2.2), so
 * the manual price drives the WeChat order amount.
 */
export function providerRequiresManualPrice(provider?: string | null): boolean {
  return provider === PAYMENT_PROVIDERS.WECHAT
}

/**
 * Whether the provider can fulfill `recurring` billing (auto-renewal).
 * WeChat has no merchant-initiated deduction in scope — recurring mappings
 * are rejected server-side, so the form hides the option up front.
 */
export function providerAllowsRecurringBilling(provider?: string | null): boolean {
  return provider !== PAYMENT_PROVIDERS.WECHAT
}

export const PLAN_TIERS = {
  FREE: 'free',
  PRO: 'pro',
  ENTERPRISE: 'enterprise',
} as const

export type PlanTier = (typeof PLAN_TIERS)[keyof typeof PLAN_TIERS]
