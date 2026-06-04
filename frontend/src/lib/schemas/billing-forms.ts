import { z } from 'zod'
import { m } from '@/paraglide/messages'

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
    .min(3, { error: () => m['billing.product_code_min_length']() })
    .max(50, { error: () => m['billing.product_code_max_length']() })
    .regex(PLAN_NAME_REGEX, { error: () => m['billing.product_code_format']() }),
  title: z
    .string()
    .min(1, { error: () => m['billing.title_required']() })
    .max(100, { error: () => m['billing.title_max_length']() }),
  description: z
    .string()
    .max(500, { error: () => m['billing.description_max_length']() })
    .optional(),
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
  productId: z.string().min(1, { error: () => m['billing.product_required']() }),
  name: z
    .string()
    .min(1, { error: () => m['billing.plan_name_required']() })
    .max(50, { error: () => m['billing.plan_name_max_length']() })
    .regex(PLAN_NAME_REGEX, { error: () => m['billing.plan_name_format']() }),
  title: z
    .string()
    .min(1, { error: () => m['billing.title_required']() })
    .max(100, { error: () => m['billing.title_max_length']() }),
  description: z
    .string()
    .max(500, { error: () => m['billing.description_max_length']() })
    .optional(),
  type: z.enum(['monthly', 'yearly']),
  price: z
    .number()
    .min(0.01, { error: () => m['billing.price_min']() })
    .max(99999.99, { error: () => m['billing.price_max']() })
    .transform((val) => Math.round(val * 100)),
  currency: z
    .string()
    .min(3, { error: () => m['billing.currency_min_length']() })
    .max(3, { error: () => m['billing.currency_max_length']() }),
  checkoutUrl: z
    .string()
    .max(2048, { error: () => m['billing.checkout_url_max_length']() })
    .optional()
    .transform((val) => (val === '' ? undefined : val))
    .refine((val) => !val || URL_SCHEMA.safeParse(val).success, {
      error: () => m['billing.checkout_url_invalid'](),
    }),
  trialDays: z
    .number()
    .min(0, { error: () => m['billing.trial_days_min']() })
    .max(365, { error: () => m['billing.trial_days_max']() })
    .default(0),
  sortOrder: z
    .number()
    .min(0, { error: () => m['billing.sort_order_min']() })
    .default(0),
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
    .min(1, { error: () => m['billing.shopify_domain_required']() })
    .regex(SHOPIFY_DOMAIN_REGEX, { error: () => m['billing.shopify_domain_format']() })
    .trim()
    .toLowerCase(),

  adminAccessToken: z
    .string()
    .trim()
    .regex(SHOPIFY_ADMIN_TOKEN_REGEX, { error: () => m['billing.shopify_admin_token_format']() })
    .or(z.literal('')),

  storefrontAccessToken: z
    .string()
    .trim()
    .regex(SHOPIFY_STOREFRONT_TOKEN_REGEX, {
      error: () => m['billing.shopify_storefront_token_format'](),
    })
    .or(z.literal('')),

  appClientSecret: z
    .string()
    .trim()
    .min(10, { error: () => m['billing.shopify_app_secret_min_length']() })
    .or(z.literal('')),

  apiVersion: z.string().default('2024-01').optional(),

  webhookSubscriptionMode: z
    .enum([WEBHOOK_MODES.ADMIN_API, WEBHOOK_MODES.EVENT_BRIDGE])
    .default(WEBHOOK_MODES.ADMIN_API)
    .optional(),

  timeout: z
    .number()
    .int()
    .min(1, { error: () => m['billing.shopify_timeout_min']() })
    .max(120, { error: () => m['billing.shopify_timeout_max']() })
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
      .refine((val) => !val || val.length > 0, {
        error: () => m['billing.claim_customer_id_empty'](),
      }),

    contractId: z
      .string()
      .optional()
      .refine((val) => !val || val.length > 0, {
        error: () => m['billing.claim_contract_id_empty'](),
      }),

    grantCurrentPeriod: z.boolean().default(true),
  })
  .refine((data) => data.shopifyCustomerId || data.contractId, {
    error: () => m['billing.claim_either_required'](),
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
  paymentProvider: z.string().min(1, { error: () => m['billing.provider_mapping_required']() }),
  externalProductId: z
    .string()
    .min(1, { error: () => m['billing.external_product_id_required']() })
    .max(255, { error: () => m['billing.external_product_id_max_length']() }),
  externalPriceId: z
    .string()
    .max(255, { error: () => m['billing.external_price_id_max_length']() })
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

  appId: z
    .string()
    .min(1, { error: () => m['billing.wechat_app_id_required']() })
    .regex(/^wx/, { error: () => m['billing.wechat_app_id_format']() })
    .trim(),

  mchId: z
    .string()
    .min(1, { error: () => m['billing.wechat_merchant_id_required']() })
    .regex(/^\d+$/, { error: () => m['billing.wechat_merchant_id_format']() })
    .trim(),

  privateKey: z
    .string()
    .refine(
      (val) =>
        !val ||
        (val.includes('-----BEGIN PRIVATE KEY-----') && val.includes('-----END PRIVATE KEY-----')),
      { error: () => m['billing.wechat_private_key_format']() }
    )
    .trim(),

  serialNo: z
    .string()
    .min(1, { error: () => m['billing.wechat_serial_no_required']() })
    .trim(),

  v3Key: z
    .string()
    .refine((val) => !val || val.length === 32, {
      error: () => m['billing.wechat_v3_key_format'](),
    })
    .trim(),

  platformPublicKey: z
    .string()
    .refine(
      (val) =>
        !val ||
        (val.includes('-----BEGIN PUBLIC KEY-----') && val.includes('-----END PUBLIC KEY-----')),
      { error: () => m['billing.wechat_platform_key_format']() }
    )
    .trim(),

  notifyUrl: z
    .string()
    .min(1, { error: () => m['billing.wechat_notify_url_required']() })
    .url({ error: () => m['billing.wechat_notify_url_format']() })
    .refine((val) => val.startsWith('https://'), {
      error: () => m['billing.wechat_notify_url_https'](),
    })
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
