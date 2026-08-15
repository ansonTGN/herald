/**
 * Currency-code validation and normalization — the frontend mirror of the
 * backend's `validate_currency_code` write-path rules.
 *
 * Lives in `lib/` (not `components/`) so schemas and other lib modules can
 * depend on it without a lib→components edge; the purchase-page grouping
 * helpers stay in `components/billing/currency-utils.ts`.
 */

/** ISO 4217 alphabetic code shape: exactly three uppercase ASCII letters. */
export const CURRENCY_CODE_PATTERN = /^[A-Z]{3}$/

/** ISO 4217 reserved codes that are not real currencies and must be rejected. */
export const RESERVED_CURRENCY_CODES: ReadonlySet<string> = new Set(['XXX', 'XTS'])

/**
 * Mirror of the backend's currency-code validation: 3-letter uppercase format
 * plus reserved-code rejection. Used by the profile and realm-settings forms
 * so an invalid code is stopped at form validation, before the request.
 */
export function isValidCurrencyCode(code: string): boolean {
  return CURRENCY_CODE_PATTERN.test(code) && !RESERVED_CURRENCY_CODES.has(code)
}

/** Normalize a currency code (or absent value) to an uppercase grouping key. */
export function normalizeCurrencyCode(code: string | null | undefined): string {
  return (code ?? '').trim().toUpperCase()
}
