import { capitalize } from '@/lib/utils'

export function formatProviderName(name: string): string {
  const providerNames: Record<string, string> = {
    wechat: 'WeChat Pay',
    stripe: 'Stripe',
    shopify: 'Shopify',
    creem: 'Creem',
  }
  return providerNames[name] || capitalize(name)
}
