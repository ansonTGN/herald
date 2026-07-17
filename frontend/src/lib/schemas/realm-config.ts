import { z } from 'zod'

// TOTP 配置 Schema
// ✅ 前端 Schema 使用 camelCase（符合 JavaScript 约定）
export const totpConfigSchema = z.object({
  enabled: z.boolean(), // Realm 是否启用 TOTP
  forceEnabled: z.boolean(), // ✅ camelCase：是否强制所有用户启用 TOTP
})

// Passkey 配置 Schema
// ✅ camelCase：对齐后端 `GetRealmPasskeyConfigResponse` /
//    `UpdateRealmPasskeyConfigRequest`（均为 camelCase 线传输）。
//    实测字段：enabled / forceEnabled（必填）+ P1 策略字段
//    userVerification / crossPlatformAuthenticator。
//    userVerification 在 wire 上为 string（后端枚举 "preferred"|"required"），
//    在此收窄为枚举并提供默认值，保证表单缺省时的可空安全。
export const passkeyConfigSchema = z.object({
  enabled: z.boolean(), // Realm 是否启用 Passkey
  forceEnabled: z.boolean(), // 是否强制引导用户注册并优先使用 Passkey（仍保留密码/TOTP 回退）
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

// 类型导出
export type TOTPConfigForm = z.infer<typeof totpConfigSchema>
export type PasskeyConfigForm = z.infer<typeof passkeyConfigSchema>
export type EmailOtpConfigForm = z.infer<typeof emailOtpConfigSchema>
export type RegistrationConfigForm = z.infer<typeof registrationConfigSchema>
export type TurnstileConfigForm = z.infer<typeof turnstileConfigSchema>
export type EmailConfigForm = z.infer<typeof emailConfigSchema>
export type WhiteLabelBackgroundForm = z.infer<typeof whiteLabelBackgroundSchema>
export type WhiteLabelConfigForm = z.infer<typeof whiteLabelConfigSchema>
export type CustomDomainConfigForm = z.infer<typeof customDomainConfigSchema>
