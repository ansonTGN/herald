/**
 * OAuth Provider Types and Constants
 */

/**
 * OAuth Provider Types
 */
export type ProviderType =
  | 'google'
  | 'github'
  | 'facebook'
  | 'apple'
  | 'wechat'
  | 'wechat_miniprogram'

/**
 * Provider types array
 */
export const PROVIDER_TYPES: readonly ProviderType[] = [
  'google',
  'github',
  'facebook',
  'apple',
  'wechat',
  'wechat_miniprogram',
] as const

/**
 * Display names for OAuth providers
 */
export const PROVIDER_DISPLAY_NAMES: Record<ProviderType, string> = {
  google: 'Google',
  github: 'GitHub',
  facebook: 'Facebook',
  apple: 'Apple',
  wechat: 'WeChat',
  wechat_miniprogram: 'WeChat Mini Program',
}

/**
 * Default OAuth scopes for each provider
 */
export const DEFAULT_SCOPES: Record<ProviderType, string[]> = {
  google: [
    'https://www.googleapis.com/auth/userinfo.profile',
    'https://www.googleapis.com/auth/userinfo.email',
  ],
  github: ['user:email'],
  facebook: ['email'],
  apple: ['name', 'email'],
  wechat: ['snsapi_login'],
  wechat_miniprogram: [],
}
