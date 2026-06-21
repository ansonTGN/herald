import { type PaymentProviderInfo } from '@/lib/api-generated'
import { formatProviderName } from '@/components/billing/format-provider-name'

/**
 * Returns all configured payment providers (every listed provider is enabled).
 */
export function getEnabledProviders(providers?: PaymentProviderInfo[]): PaymentProviderInfo[] {
  return providers ?? []
}

/**
 * Formats payment provider names for display in a comma-separated list
 */
export function formatProviderNames(providers: PaymentProviderInfo[]): string {
  return providers.map((p) => formatProviderName(p.platform)).join(', ')
}

/**
 * Gets formatted names of enabled providers
 */
export function getEnabledProviderNames(providers?: PaymentProviderInfo[]): string {
  return formatProviderNames(getEnabledProviders(providers))
}

/**
 * Mask a secret value for display
 * @param value - The secret value to mask
 * @param show - Whether to show the actual value
 * @param visibleChars - Number of characters to show when masked (default: 4)
 * @returns Masked string or original value if show=true
 */
export function maskSecret(value: string, show: boolean, visibleChars = 4): string {
  if (show || !value) return value
  const visible = value.substring(0, Math.min(visibleChars, value.length))
  const maskedLength = Math.max(20, value.length - visibleChars)
  return `${visible}${'*'.repeat(maskedLength)}`
}

/**
 * Mask a configured value for display
 * @param value - The value to mask
 * @param show - Whether to show the actual value
 * @returns "*********** (configured)" if value exists and show=false, otherwise the value or "Not configured"
 */
export function maskConfiguredValue(value: string, show: boolean): string {
  if (show) return value
  return value ? '*********** (configured)' : 'Not configured'
}
