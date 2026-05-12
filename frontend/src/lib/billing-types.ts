/**
 * Payment provider types enum
 * Provides type-safe payment provider identifiers
 */
export enum PaymentProvider {
  STRIPE = 'stripe',
  SHOPIFY = 'shopify',
  WECHAT = 'wechat',
  CREEM = 'creem',
}

/**
 * Type guard to check if a string is a valid payment provider
 */
export function isPaymentProvider(value: string): value is PaymentProvider {
  return Object.values(PaymentProvider).includes(value as PaymentProvider)
}

/**
 * Get display name for a payment provider
 */
export function getProviderDisplayName(provider: PaymentProvider | string): string {
  const displayNames: Record<PaymentProvider, string> = {
    [PaymentProvider.STRIPE]: 'Stripe',
    [PaymentProvider.SHOPIFY]: 'Shopify',
    [PaymentProvider.WECHAT]: 'WeChat Pay',
    [PaymentProvider.CREEM]: 'Creem',
  }

  if (isPaymentProvider(provider)) {
    return displayNames[provider]
  }

  // Fallback to capitalized form for unknown providers
  return provider.charAt(0).toUpperCase() + provider.slice(1)
}
