import { capitalize } from '@/lib/utils'

const PROVIDER_NAMES: Record<string, string> = {
  stripe: 'Stripe',
  creem: 'Creem',
  apple: 'App Store',
  google: 'Google Play',
  wechat: 'WeChat Pay',
}

export function formatProviderName(name: string): string {
  return PROVIDER_NAMES[name] || capitalize(name)
}
