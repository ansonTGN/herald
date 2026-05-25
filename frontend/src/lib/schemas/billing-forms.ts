import { z } from 'zod'

// Pre-compiled regex patterns for validation (compiled once at module load)
const PLAN_NAME_REGEX = /^[a-z0-9_-]+$/
const SHOPIFY_DOMAIN_REGEX = /\.myshopify\.com$/
const SHOPIFY_ADMIN_TOKEN_REGEX = /^shpat_/
const SHOPIFY_STOREFRONT_TOKEN_REGEX = /^shp_/
const URL_SCHEMA = z.string().url()

// Webhook subscription mode constants
export const WEBHOOK_MODES = {
  ADMIN_API: 'admin_api',
  EVENT_BRIDGE: 'event_bridge',
} as const

export type WebhookMode = (typeof WEBHOOK_MODES)[keyof typeof WEBHOOK_MODES]

// ==================== Product Form Schema ====================

export const productFormSchema = z.object({
  code: z
    .string()
    .min(3, 'Product code must be at least 3 characters')
    .max(50, 'Product code must not exceed 50 characters')
    .regex(
      PLAN_NAME_REGEX,
      'Product code can only contain lowercase letters, numbers, hyphens, and underscores'
    ),
  title: z.string().min(1, 'Title is required').max(100, 'Title must not exceed 100 characters'),
  description: z.string().max(500, 'Description must not exceed 500 characters').optional(),
  enabled: z.boolean().default(true),
})

export type ProductFormData = z.infer<typeof productFormSchema>

export function getProductDefaults(product?: Partial<ProductFormData>): ProductFormData {
  return {
    code: '',
    title: '',
    description: '',
    enabled: true,
    ...product,
  } as ProductFormData
}

// ==================== Subscription Plan Form Schema ====================

export const subscriptionPlanSchema = z.object({
  productId: z.string().min(1, 'Product is required'),
  name: z
    .string()
    .min(1, 'Plan name is required')
    .max(50, 'Plan name must not exceed 50 characters')
    .regex(
      PLAN_NAME_REGEX,
      'Plan name can only contain lowercase letters, numbers, hyphens, and underscores'
    ),
  title: z.string().min(1, 'Title is required').max(100, 'Title must not exceed 100 characters'),
  description: z.string().max(500, 'Description must not exceed 500 characters').optional(),
  type: z.enum(['monthly', 'yearly']),
  price: z
    .number()
    .min(0.01, 'Price must be at least $0.01')
    .max(99999.99, 'Price must not exceed $99,999.99')
    .transform((val) => Math.round(val * 100)),
  currency: z
    .string()
    .min(3, 'Currency must be at least 3 characters')
    .max(3, 'Currency must be exactly 3 characters'),
  checkoutUrl: z
    .string()
    .max(2048, 'Checkout URL must not exceed 2048 characters')
    .optional()
    .transform((val) => (val === '' ? undefined : val))
    .refine((val) => !val || URL_SCHEMA.safeParse(val).success, 'Invalid URL format'),
  trialDays: z
    .number()
    .min(0, 'Trial days cannot be negative')
    .max(365, 'Trial days must not exceed 365')
    .default(0),
  sortOrder: z.number().min(0, 'Sort order cannot be negative').default(0),
  active: z.boolean().default(true),
})

export type SubscriptionPlanFormData = z.infer<typeof subscriptionPlanSchema>

export function getSubscriptionPlanDefaults(plan?: {
  productId?: string
  name?: string
  title?: string
  description?: string | null
  type?: string
  price?: number
  currency?: string
  checkoutUrl?: string | null
  trialDays?: number
  sortOrder?: number
  active?: boolean
}): SubscriptionPlanFormData {
  return {
    productId: '',
    name: '',
    title: '',
    description: '',
    type: 'monthly',
    currency: 'USD',
    checkoutUrl: undefined,
    trialDays: 0,
    sortOrder: 0,
    active: true,
    ...plan,
    price: plan?.price != null ? plan.price / 100 : undefined,
  } as SubscriptionPlanFormData
}

// ==================== Shopify Config Schema ====================

export const shopifyConfigSchema = z.object({
  shopDomain: z
    .string()
    .min(1, 'Shop Domain is required')
    .regex(SHOPIFY_DOMAIN_REGEX, 'Shop Domain must end with .myshopify.com')
    .trim()
    .toLowerCase(),

  adminAccessToken: z
    .string()
    .trim()
    .regex(SHOPIFY_ADMIN_TOKEN_REGEX, 'Must start with shpat_')
    .or(z.literal('')),

  storefrontAccessToken: z
    .string()
    .trim()
    .regex(SHOPIFY_STOREFRONT_TOKEN_REGEX, 'Must start with shp_')
    .or(z.literal('')),

  appClientSecret: z
    .string()
    .trim()
    .min(10, 'App Client Secret must be at least 10 characters')
    .or(z.literal('')),

  apiVersion: z.string().default('2024-01').optional(),

  webhookSubscriptionMode: z
    .enum([WEBHOOK_MODES.ADMIN_API, WEBHOOK_MODES.EVENT_BRIDGE])
    .default(WEBHOOK_MODES.ADMIN_API)
    .optional(),

  timeout: z
    .number()
    .int()
    .min(1, 'Timeout must be at least 1 second')
    .max(120, 'Timeout must not exceed 120 seconds')
    .default(30)
    .optional(),

  skipConnectionTest: z.boolean().default(false),
})

export type ShopifyConfigForm = z.infer<typeof shopifyConfigSchema>

export function getShopifyConfigDefaults(config?: Partial<ShopifyConfigForm>): ShopifyConfigForm {
  return {
    shopDomain: '',
    adminAccessToken: '',
    storefrontAccessToken: '',
    appClientSecret: '',
    apiVersion: '2024-01',
    webhookSubscriptionMode: WEBHOOK_MODES.ADMIN_API,
    timeout: 30,
    skipConnectionTest: false,
    ...config,
  } as ShopifyConfigForm
}

// ==================== Claim Subscription Schema ====================

export const claimSubscriptionSchema = z
  .object({
    shopifyCustomerId: z
      .string()
      .optional()
      .refine((val) => !val || val.length > 0, 'Customer ID cannot be empty'),

    contractId: z
      .string()
      .optional()
      .refine((val) => !val || val.length > 0, 'Contract ID cannot be empty'),

    grantCurrentPeriod: z.boolean().default(true),
  })
  .refine((data) => data.shopifyCustomerId || data.contractId, {
    message: 'Either Shopify Customer ID or Contract ID is required',
    path: ['shopifyCustomerId'],
  })

export type ClaimSubscriptionForm = z.infer<typeof claimSubscriptionSchema>

export function getClaimSubscriptionDefaults(): ClaimSubscriptionForm {
  return {
    shopifyCustomerId: '',
    contractId: '',
    grantCurrentPeriod: true,
  } as ClaimSubscriptionForm
}

// ==================== Provider Mapping Schema ====================

export const providerMappingSchema = z.object({
  paymentProvider: z.string().min(1, 'Payment provider is required'),
  externalProductId: z
    .string()
    .min(1, 'External product ID is required')
    .max(255, 'External product ID must not exceed 255 characters'),
  externalPriceId: z
    .string()
    .max(255, 'External price ID must not exceed 255 characters')
    .optional(),
  enabled: z.boolean().optional().default(true),
})

export type ProviderMappingFormData = z.infer<typeof providerMappingSchema>

export function getProviderMappingDefaults(mapping?: {
  paymentProvider?: string
  externalProductId?: string
  externalPriceId?: string | null
  enabled?: boolean
}): ProviderMappingFormData {
  return {
    paymentProvider: '',
    externalProductId: '',
    externalPriceId: '',
    enabled: true,
    ...mapping,
  } as ProviderMappingFormData
}

// ==================== WeChat Config Schema ====================

export const wechatConfigSchema = z.object({
  enabled: z.boolean().default(true),

  appId: z.string().min(1, 'App ID is required').regex(/^wx/, 'App ID must start with "wx"').trim(),

  mchId: z
    .string()
    .min(1, 'Merchant ID is required')
    .regex(/^\d+$/, 'Merchant ID must be numeric')
    .trim(),

  privateKey: z
    .string()
    .refine(
      (val) =>
        !val ||
        (val.includes('-----BEGIN PRIVATE KEY-----') && val.includes('-----END PRIVATE KEY-----')),
      'Private Key must be in valid PEM format'
    )
    .trim(),

  serialNo: z.string().min(1, 'Serial No is required').trim(),

  v3Key: z
    .string()
    .refine((val) => !val || val.length === 32, 'API v3 Key must be exactly 32 bytes')
    .trim(),

  platformPublicKey: z
    .string()
    .refine(
      (val) =>
        !val ||
        (val.includes('-----BEGIN PUBLIC KEY-----') && val.includes('-----END PUBLIC KEY-----')),
      'Platform Public Key must be in valid PEM format'
    )
    .trim(),

  notifyUrl: z
    .string()
    .min(1, 'Notify URL is required')
    .url('Invalid URL format')
    .refine((val) => val.startsWith('https://'), 'Notify URL must use HTTPS')
    .trim(),
})

export type WechatConfigForm = z.infer<typeof wechatConfigSchema>

export function getWechatConfigDefaults(config?: Partial<WechatConfigForm>): WechatConfigForm {
  return {
    enabled: true,
    appId: '',
    mchId: '',
    privateKey: '',
    serialNo: '',
    v3Key: '',
    platformPublicKey: '',
    notifyUrl: '',
    ...config,
  } as WechatConfigForm
}
