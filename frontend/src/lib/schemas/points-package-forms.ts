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

/**
 * Calculate discount percentage between selling price and original price.
 * Returns 0-100 integer representing how much percent off.
 */
export function calculateDiscountPercent(price: number, originalPrice: number): number {
  return Math.round((1 - price / originalPrice) * 100)
}

// ==================== Promo Cross-Field Validation ====================

/**
 * Shared refine function for promo package cross-field validation.
 * Applied independently to both create and update schemas because
 * .superRefine() returns ZodEffects which lacks .partial().
 */
function promoRefine<
  T extends {
    packageType?: string
    price?: number
    originalPrice?: number
    promoStartTime?: string
    promoEndTime?: string
  },
>(data: T, ctx: z.RefinementCtx): void {
  if (data.packageType === 'promotional') {
    if (data.originalPrice == null || data.originalPrice === undefined) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'Original price is required for promotional packages',
        path: ['originalPrice'],
      })
    } else if (data.originalPrice <= (data.price ?? 0)) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'Original price must be greater than the selling price',
        path: ['originalPrice'],
      })
    }

    if (!data.promoEndTime) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        message: 'Promo end time is required for promotional packages',
        path: ['promoEndTime'],
      })
    }
  }

  if (data.promoStartTime && data.promoEndTime && data.promoEndTime <= data.promoStartTime) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      message: 'Promo end time must be after start time',
      path: ['promoEndTime'],
    })
  }
}

// ==================== Points Package Form Schema ====================

// Store raw ZodObject before superRefine so update schema can use .partial()
const _pointsPackageBaseSchema = z.object({
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
  packageType: z.enum(['standard', 'promotional']).default('standard'),
  originalPrice: z
    .number()
    .min(0.01, 'Original price must be at least 0.01')
    .max(9999999, 'Original price must not exceed 9999999')
    .optional(),
  promoStartTime: z.string().optional(),
  promoEndTime: z.string().optional(),
})

export const pointsPackageFormSchema = _pointsPackageBaseSchema.superRefine(promoRefine)

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
  packageType?: string
  originalPrice?: number
  promoStartTime?: string
  promoEndTime?: string
}): PointsPackageFormData {
  const { originalPrice: _apiOriginalPrice, currency: _currency, ...rest } = pkg ?? {}
  return {
    name: '',
    title: '',
    description: '',
    points: 100,
    price: 9.99,
    currency: 'USD',
    sortOrder: 0,
    enabled: true,
    packageType: 'standard',
    promoStartTime: '',
    promoEndTime: '',
    ...rest,
    originalPrice:
      _apiOriginalPrice != null
        ? apiPriceToDisplayPrice(_apiOriginalPrice, _currency ?? 'USD')
        : undefined,
  } as PointsPackageFormData
}

// ==================== Update Points Package Form Schema ====================

// For updates, points field is immutable and should not be included
export const updatePointsPackageFormSchema = _pointsPackageBaseSchema
  .partial()
  .extend({
    // Points field is intentionally excluded - it's immutable
    name: _pointsPackageBaseSchema.shape.name.optional(),
    title: _pointsPackageBaseSchema.shape.title.optional(),
    description: _pointsPackageBaseSchema.shape.description.optional(),
    price: _pointsPackageBaseSchema.shape.price.optional(),
    currency: _pointsPackageBaseSchema.shape.currency.optional(),
    sortOrder: _pointsPackageBaseSchema.shape.sortOrder.optional(),
    enabled: _pointsPackageBaseSchema.shape.enabled.optional(),
    packageType: _pointsPackageBaseSchema.shape.packageType.optional(),
    originalPrice: _pointsPackageBaseSchema.shape.originalPrice.optional(),
    promoStartTime: _pointsPackageBaseSchema.shape.promoStartTime.optional(),
    promoEndTime: _pointsPackageBaseSchema.shape.promoEndTime.optional(),
  })
  .superRefine(promoRefine)

export type UpdatePointsPackageFormData = z.infer<typeof updatePointsPackageFormSchema>

export function getUpdatePointsPackageDefaults(pkg: {
  name?: string
  title?: string
  description?: string | null
  price?: number
  currency?: string
  sortOrder?: number
  enabled?: boolean
  packageType?: string
  originalPrice?: number
  promoStartTime?: string
  promoEndTime?: string
}): UpdatePointsPackageFormData {
  return {
    name: pkg.name ?? '',
    title: pkg.title ?? '',
    description: pkg.description ?? '',
    price: pkg.price ?? 9.99,
    currency: pkg.currency ?? 'USD',
    sortOrder: pkg.sortOrder ?? 0,
    enabled: pkg.enabled ?? true,
    packageType: pkg.packageType ?? 'standard',
    originalPrice:
      pkg.originalPrice != null
        ? apiPriceToDisplayPrice(pkg.originalPrice, pkg.currency ?? 'USD')
        : undefined,
    promoStartTime: pkg.promoStartTime ?? '',
    promoEndTime: pkg.promoEndTime ?? '',
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
