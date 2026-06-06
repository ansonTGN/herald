import { z } from 'zod'
import { m } from '@/paraglide/messages'

// Pre-compiled regex patterns for validation (compiled once at module load)
const SHOPIFY_DOMAIN_REGEX = /\.myshopify\.com$/
const SHOPIFY_ADMIN_TOKEN_REGEX = /^shpat_/
const SHOPIFY_STOREFRONT_TOKEN_REGEX = /^shp_/

// Webhook subscription mode constants
export const WEBHOOK_MODES = {
  ADMIN_API: 'admin_api',
  EVENT_BRIDGE: 'event_bridge',
} as const

export type WebhookMode = (typeof WEBHOOK_MODES)[keyof typeof WEBHOOK_MODES]

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

// ==================== Entitlement Mapping Update Schema ====================

const ENTITLEMENT_KEY_REGEX = /^[a-z0-9-]{1,64}$/

export const entitlementMappingUpdateSchema = z.object({
  entitlementKey: z
    .string()
    .min(1, { error: () => m['billing.entitlement_key_required']() })
    .regex(ENTITLEMENT_KEY_REGEX, { error: () => m['billing.entitlement_key_format']() }),

  enabled: z.boolean().default(false),

  pointsPerPeriod: z.number().int().min(0).optional().nullable(),

  grantPeriodType: z.enum(['once', 'daily', 'weekly', 'monthly']).optional().nullable(),

  validityDays: z.number().int().min(1).optional().nullable(),

  grantOnSubscribe: z.boolean().default(false),

  maxPeriods: z.number().int().min(1).optional().nullable(),
})

export type EntitlementMappingUpdateFormData = z.infer<typeof entitlementMappingUpdateSchema>

export function getEntitlementMappingUpdateDefaults(
  config?: Partial<EntitlementMappingUpdateFormData>
): EntitlementMappingUpdateFormData {
  return {
    entitlementKey: '',
    enabled: false,
    pointsPerPeriod: null,
    grantPeriodType: null,
    validityDays: null,
    grantOnSubscribe: false,
    maxPeriods: null,
    ...config,
  } as EntitlementMappingUpdateFormData
}
