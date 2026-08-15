import { z } from 'zod'
import { isValidCurrencyCode } from '@/lib/currency-utils'

// TOTP 配置 Schema
// ✅ 前端 Schema 使用 camelCase（符合 JavaScript 约定）
export const totpConfigSchema = z.object({
  enabled: z.boolean(), // Realm 是否启用 TOTP
  forceEnabled: z.boolean(), // ✅ camelCase：是否强制所有用户启用 TOTP
})

// Passkey 配置 Schema
// ✅ camelCase：对齐后端 `GetRealmPasskeyConfigResponse` /
//    `UpdateRealmPasskeyConfigRequest`（均为 camelCase 线传输）。
//    实测字段：enabled（必填）+ P1 策略字段
//    userVerification / crossPlatformAuthenticator。
//    userVerification 在 wire 上为 string（后端枚举 "preferred"|"required"），
//    在此收窄为枚举并提供默认值，保证表单缺省时的可空安全。
export const passkeyConfigSchema = z.object({
  enabled: z.boolean(), // Realm 是否启用 Passkey
  userVerification: z.enum(['preferred', 'required']).default('preferred'), // P1：用户验证要求
  crossPlatformAuthenticator: z.boolean().default(true), // P1：是否要求跨平台 authenticator
})

// Email-OTP 配置 Schema
// ✅ camelCase：对齐后端 `GetRealmEmailOtpConfigResponse` /
//    `UpdateRealmEmailOtpConfigRequest`（均为 camelCase 线传输）。
//    实测字段：enabled（是否启用邮箱验证码登录）/
//    autoRegister（未注册邮箱验证成功后是否自动注册并激活账户）。
export const emailOtpConfigSchema = z.object({
  enabled: z.boolean(), // Realm 是否启用邮箱验证码登录
  autoRegister: z.boolean(), // ✅ camelCase：未注册邮箱是否自动注册
})

// Registration 配置 Schema
export const registrationConfigSchema = z.object({
  enabled: z.boolean(), // 是否允许注册
  requireEmailVerification: z.boolean(), // ✅ camelCase：是否需要邮箱验证
})

// Platform self-service signup 配置 Schema
// ✅ admin realm 独有：控制公开自助开通 Realm 的总闸 (DEC-009/013)。
//    存于 realm_config(platform_signup, enabled) 单行，缺失 = false (fail-closed)。
export const platformSignupConfigSchema = z.object({
  enabled: z.boolean(), // 是否允许公开自助开通 Realm
})

// Turnstile 配置 Schema
export const turnstileConfigSchema = z.object({
  siteKey: z.string(),
  secretKey: z.string(),
})

// Email 配置 Schema
export const emailConfigSchema = z.object({
  provider: z.enum(['resend', 'smtp']),
  fromAddress: z.string().email().or(z.literal('')),
  resendApiKey: z.string().optional(),
  smtpHost: z.string().optional(),
  smtpPort: z.string().default('587'),
  smtpUsername: z.string().optional(),
  smtpPassword: z.string().optional(),
  smtpEncryption: z.enum(['starttls', 'ssl']).default('starttls'),
})

// White-label 背景配置 Schema
// ✅ camelCase：对齐后端 `WhiteLabelBackground` / `WhiteLabelBackgroundType`
//    （均为 camelCase 线传输）。`type` 对应 wire 上的 "image" | "gradient"。
export const whiteLabelBackgroundSchema = z.object({
  type: z.enum(['image', 'gradient']),
  value: z.string(),
})

// White-label 配置 Schema
// ✅ camelCase：对齐后端 `WhiteLabelConfig` / `UpdateWhiteLabelConfigRequest`
//    （均为 camelCase 线传输）。表单允许 `null` 或空字符串，保存时空字符串
//    normalize 为 `null`（见 realm-config-utils 的 toUpdateWhiteLabelConfigRequest）。
export const whiteLabelConfigSchema = z.object({
  brandName: z.string().nullable(),
  logoUrl: z.string().nullable(),
  faviconUrl: z.string().nullable(),
  accentColor: z.string().nullable(),
  background: whiteLabelBackgroundSchema.nullable(),
  footerText: z.string().nullable(),
  loginTitle: z.string().nullable(),
  loginSubtitle: z.string().nullable(),
  registerTitle: z.string().nullable(),
  registerSubtitle: z.string().nullable(),
})

// Custom-domain 配置 Schema
// ✅ camelCase：对齐后端 `CustomDomainConfig` / `UpdateCustomDomainConfigRequest`
//    （均为 camelCase 线传输）。`hostname` 为精确域名（如 `login.acme.com`），
//    表单允许 `null` 或空字符串，保存时空字符串 normalize 为 `null`
//    （见 realm-config-utils 的 toUpdateCustomDomainConfigRequest）。
//    刻意使用 z.string()（而非 .email()），格式校验留给后端，mapper 仅 trim。
export const customDomainConfigSchema = z.object({
  hostname: z.string().nullable(),
})

// Billing 配置 Schema（Realm 默认货币）
// ✅ 存于 realm_config(billing/default_currency) 单行字符串值；空字符串表示
//    尚未配置。非空值必须是 3 位大写 ISO 4217 码且非保留码（XXX/XTS），
//    与后端写入路径的校验一致；空值允许通过 schema（表单在提交门控中拦截）。
export const billingCurrencyConfigSchema = z.object({
  defaultCurrency: z.string().refine((v) => v === '' || isValidCurrencyCode(v), {
    message: 'invalid ISO 4217 currency code',
  }),
})

// 类型导出
export type TOTPConfigForm = z.infer<typeof totpConfigSchema>
export type PasskeyConfigForm = z.infer<typeof passkeyConfigSchema>
export type EmailOtpConfigForm = z.infer<typeof emailOtpConfigSchema>
export type RegistrationConfigForm = z.infer<typeof registrationConfigSchema>
export type PlatformSignupConfigForm = z.infer<typeof platformSignupConfigSchema>
export type TurnstileConfigForm = z.infer<typeof turnstileConfigSchema>
export type EmailConfigForm = z.infer<typeof emailConfigSchema>
export type WhiteLabelBackgroundForm = z.infer<typeof whiteLabelBackgroundSchema>
export type WhiteLabelConfigForm = z.infer<typeof whiteLabelConfigSchema>
export type CustomDomainConfigForm = z.infer<typeof customDomainConfigSchema>
export type BillingCurrencyConfigForm = z.infer<typeof billingCurrencyConfigSchema>
