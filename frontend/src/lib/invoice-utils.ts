import type { InvoiceLineItemFormData } from '@/lib/schemas/invoice-forms'
import { m } from '@/paraglide/messages'

export const INVOICE_PAGE_SIZE = 20

export const PDF_DOWNLOADABLE_STATUSES = new Set(['issued', 'paid', 'overdue'])

export function getInvoiceStatusLabel(status: string): string {
  const labels: Record<string, string> = {
    draft: m['billing.invoice_status_draft'](),
    issued: m['billing.invoice_status_issued'](),
    paid: m['billing.invoice_status_paid'](),
    void: m['billing.invoice_status_void'](),
    overdue: m['billing.invoice_status_overdue'](),
  }
  return labels[status] ?? status
}

export const INVOICE_STATUS_COLORS: Record<string, string> = {
  draft: 'secondary',
  issued: 'default',
  void: 'destructive',
}

export function getInvoiceSourceLabel(source: string): string {
  const labels: Record<string, string> = {
    admin_manual: m['billing.invoice_source_manual'](),
    user_application: m['billing.invoice_source_application'](),
  }
  return labels[source] ?? source
}

export function formatInvoiceAmount(amount: number, currency: string): string {
  const value = amount / 100
  try {
    return new Intl.NumberFormat(undefined, {
      style: 'currency',
      currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    }).format(value)
  } catch {
    return `${currency.toUpperCase()} ${value.toFixed(2)}`
  }
}

export function displayPriceToCents(value: string): number {
  return Math.round(parseFloat(value) * 100)
}

export function centsToDisplayPrice(cents: number): string {
  return (cents / 100).toFixed(2)
}

export function downloadInvoicePdf(url: string, filename: string) {
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  document.body.appendChild(link)
  try {
    link.click()
  } finally {
    document.body.removeChild(link)
  }
}

export type InvoiceAction = 'view' | 'edit' | 'issue' | 'void' | 'markPaid' | 'downloadPdf'

const ACTION_RULES: Record<string, InvoiceAction[]> = {
  draft: ['view', 'edit', 'issue', 'void'],
  issued: ['view', 'void', 'markPaid', 'downloadPdf'],
  paid: ['view', 'downloadPdf'],
  overdue: ['view', 'void', 'markPaid', 'downloadPdf'],
  void: ['view'],
}

export function isExternalInvoice(provider: string | undefined): boolean {
  return !!provider && provider !== 'manual'
}

export function getProviderLabel(provider: string): string {
  switch (provider) {
    case 'manual':
      return m['billing.invoice_provider_manual']()
    case 'wechat':
      return m['billing.invoice_provider_wechat']()
    case 'stripe':
      return 'Stripe'
    case 'creem':
      return 'Creem'
    case 'shopify':
      return 'Shopify'
    default:
      return provider
  }
}

export function getViewInProviderUrl(invoice: {
  provider: string
  externalHostedUrl?: string | null
}): string | null {
  if (invoice.provider === 'manual') return null
  return invoice.externalHostedUrl ?? null
}

export function getAvailableActions(status: string, provider?: string): InvoiceAction[] {
  if (provider && provider !== 'manual') return ['view']
  return ACTION_RULES[status] ?? ['view']
}

/**
 * Parse payment terms string into number of days.
 * "Net 30" → 30, "Net 60" → 60, "Due on Receipt" → 0
 * Returns undefined if parsing fails.
 */
export function parsePaymentTermsDays(terms: string | null | undefined): number | undefined {
  if (!terms) return undefined
  const netMatch = /Net (\d+)/i.exec(terms)
  if (netMatch) return parseInt(netMatch[1], 10)
  if (/Due on Receipt/i.test(terms)) return 0
  return undefined
}

export function calculateLineSubtotal(quantity: string, unitPrice: string): number {
  const qty = parseFloat(quantity)
  const price = parseFloat(unitPrice)
  if (isNaN(qty) || qty <= 0 || isNaN(price)) return 0
  return Math.round(qty * price * 100)
}

interface ModifierInput {
  discountMode?: string | null
  discountValue?: string | null
  taxMode?: string | null
  taxValue?: string | null
  shippingMode?: string | null
  shippingValue?: string | null
}

interface CalculatedTotals {
  subtotal: number
  discountAmount: number
  taxAmount: number
  shippingAmount: number
  total: number
}

export function calculateTotals(
  lineItems: Pick<InvoiceLineItemFormData, 'quantity' | 'unitPrice'>[],
  modifiers: ModifierInput
): CalculatedTotals {
  const {
    discountMode: dm,
    discountValue: dv,
    taxMode: tm,
    taxValue: tv,
    shippingMode: sm,
    shippingValue: sv,
  } = modifiers

  const subtotal = lineItems.reduce(
    (sum, item) => sum + calculateLineSubtotal(item.quantity, item.unitPrice),
    0
  )

  let discountAmount = 0
  if (dm && dv) {
    const parsed = parseFloat(dv)
    if (!isNaN(parsed) && parsed >= 0) {
      if (dm === 'fixed') {
        discountAmount = Math.round(parsed * 100)
      } else if (dm === 'percent') {
        discountAmount = Math.round((subtotal * parsed) / 100)
      }
    }
  }

  let taxAmount = 0
  const afterDiscount = subtotal - discountAmount
  if (tm && tv) {
    const parsed = parseFloat(tv)
    if (!isNaN(parsed) && parsed >= 0) {
      if (tm === 'fixed') {
        taxAmount = Math.round(parsed * 100)
      } else if (tm === 'percent') {
        taxAmount = Math.round((afterDiscount * parsed) / 100)
      }
    }
  }

  let shippingAmount = 0
  if (sm && sv) {
    const parsed = parseFloat(sv)
    if (!isNaN(parsed) && parsed >= 0) {
      shippingAmount = Math.round(parsed * 100)
    }
  }

  const total = Math.max(0, afterDiscount + taxAmount + shippingAmount)

  return { subtotal, discountAmount, taxAmount, shippingAmount, total }
}
