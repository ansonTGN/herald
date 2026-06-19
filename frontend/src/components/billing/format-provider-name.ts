import { capitalize } from '@/lib/utils'

export function formatProviderName(name: string): string {
  const providerNames: Record<string, string> = {
    wechat: 'WeChat Pay',
    stripe: 'Stripe',
    creem: 'Creem',
  }
  return providerNames[name] || capitalize(name)
}
