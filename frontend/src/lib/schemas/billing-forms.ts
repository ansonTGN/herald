import { z } from 'zod'
import { m } from '@/paraglide/messages'

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
