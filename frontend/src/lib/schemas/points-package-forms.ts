import { z } from 'zod'

// ==================== Payment Provider Constants ====================

/**
 * Payment provider constants with type-safe branding.
 * Prevents typos and provides autocomplete support.
 */
export const PAYMENT_PROVIDERS = {
  WECHAT: 'wechat',
  STRIPE: 'stripe',
  CREEM: 'creem',
} as const

export type PaymentProvider = (typeof PAYMENT_PROVIDERS)[keyof typeof PAYMENT_PROVIDERS]

// Zod schema for payment provider validation
export const paymentProviderSchema = z.enum(['wechat', 'stripe', 'creem'])

// Pre-compiled regex pattern for validation
const PACKAGE_NAME_REGEX = /^[a-z0-9_-]+$/

// ==================== Points Package Form Schema ====================

export const pointsPackageFormSchema = z.object({
  name: z
    .string()
    .min(3, 'Package name must be at least 3 characters')
    .max(50, 'Package name must not exceed 50 characters')
    .regex(
      PACKAGE_NAME_REGEX,
      'Package name can only contain lowercase letters, numbers, hyphens, and underscores'
    ),
  title: z.string().min(1, 'Title is required').max(100, 'Title must not exceed 100 characters'),
  description: z.string().max(500, 'Description must not exceed 500 characters').optional(),
  points: z.number().int('Points must be an integer').min(1, 'Points must be at least 1'),
  price: z
    .number()
    .min(0.01, 'Price must be at least 0.01')
    .max(9999999, 'Price must not exceed 9999999'),
  currency: z
    .string()
    .length(3, 'Currency must be exactly 3 characters (ISO 4217)')
    .regex(/^[A-Z]+$/, 'Currency must be uppercase ISO 4217 code'),
  sortOrder: z
    .number()
    .int('Sort order must be an integer')
    .min(0, 'Sort order cannot be negative')
    .default(0),
  enabled: z.boolean().default(true),
})

export type PointsPackageFormData = z.infer<typeof pointsPackageFormSchema>

export function getPointsPackageDefaults(pkg?: {
  name?: string
  title?: string
  description?: string | null
  points?: number
  price?: number
  currency?: string
  sortOrder?: number
  enabled?: boolean
}): PointsPackageFormData {
  return {
    name: '',
    title: '',
    description: '',
    points: 100,
    price: 9.99,
    currency: 'USD',
    sortOrder: 0,
    enabled: true,
    ...pkg,
  } as PointsPackageFormData
}

// ==================== Update Points Package Form Schema ====================

// For updates, points field is immutable and should not be included
export const updatePointsPackageFormSchema = pointsPackageFormSchema.partial().extend({
  // Points field is intentionally excluded - it's immutable
  name: pointsPackageFormSchema.shape.name.optional(),
  title: pointsPackageFormSchema.shape.title.optional(),
  description: pointsPackageFormSchema.shape.description.optional(),
  price: pointsPackageFormSchema.shape.price.optional(),
  currency: pointsPackageFormSchema.shape.currency.optional(),
  sortOrder: pointsPackageFormSchema.shape.sortOrder.optional(),
  enabled: pointsPackageFormSchema.shape.enabled.optional(),
})

export type UpdatePointsPackageFormData = z.infer<typeof updatePointsPackageFormSchema>

export function getUpdatePointsPackageDefaults(pkg: {
  name?: string
  title?: string
  description?: string | null
  price?: number
  currency?: string
  sortOrder?: number
  enabled?: boolean
}): UpdatePointsPackageFormData {
  return {
    name: pkg.name ?? '',
    title: pkg.title ?? '',
    description: pkg.description ?? '',
    price: pkg.price ?? 9.99,
    currency: pkg.currency ?? 'USD',
    sortOrder: pkg.sortOrder ?? 0,
    enabled: pkg.enabled ?? true,
  } as UpdatePointsPackageFormData
}

// ==================== Payment Provider Mapping Schema ====================

export const paymentProviderMappingSchema = z.object({
  paymentProvider: paymentProviderSchema,
  enabled: z.boolean().default(true),
  externalProductId: z
    .string()
    .max(255, 'External product ID must not exceed 255 characters')
    .optional(),
})

export type PaymentProviderMappingFormData = z.infer<typeof paymentProviderMappingSchema>

export function getPaymentProviderMappingDefaults(mapping?: {
  paymentProvider?: string
  enabled?: boolean
  externalProductId?: string | null
}): PaymentProviderMappingFormData {
  return {
    paymentProvider: PAYMENT_PROVIDERS.STRIPE,
    enabled: true,
    externalProductId: '',
    ...mapping,
  } as PaymentProviderMappingFormData
}

// ==================== Price Conversion Utilities ====================

/**
 * Currency decimal places (ISO 4217 standard)
 */
const CURRENCY_DECIMAL_PLACES: Record<string, number> = {
  USD: 2,
  EUR: 2,
  GBP: 2,
  CNY: 2,
  JPY: 0,
}

/**
 * Convert display price (9.99 USD) to API price (999 cents)
 */
export function displayPriceToApiPrice(displayPrice: number, currency: string): number {
  const decimalPlaces = CURRENCY_DECIMAL_PLACES[currency] ?? 2
  const multiplier = Math.pow(10, decimalPlaces)
  return Math.round(displayPrice * multiplier)
}

/**
 * Convert API price (999 cents) to display price (9.99 USD)
 */
export function apiPriceToDisplayPrice(apiPrice: number, currency: string): number {
  const decimalPlaces = CURRENCY_DECIMAL_PLACES[currency] ?? 2
  const divisor = Math.pow(10, decimalPlaces)
  return apiPrice / divisor
}

/**
 * Format price for display: "9.99 USD"
 */
export function formatPrice(apiPrice: number, currency: string): string {
  const displayPrice = apiPriceToDisplayPrice(apiPrice, currency)
  const decimalPlaces = CURRENCY_DECIMAL_PLACES[currency] ?? 2
  return `${displayPrice.toFixed(decimalPlaces)} ${currency}`
}
