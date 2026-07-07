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

// 类型导出
export type TOTPConfigForm = z.infer<typeof totpConfigSchema>
export type PasskeyConfigForm = z.infer<typeof passkeyConfigSchema>
export type RegistrationConfigForm = z.infer<typeof registrationConfigSchema>
export type TurnstileConfigForm = z.infer<typeof turnstileConfigSchema>
export type EmailConfigForm = z.infer<typeof emailConfigSchema>
