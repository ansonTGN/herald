import type { InvoiceLineItemFormData } from '@/lib/schemas/invoice-forms'

export const INVOICE_PAGE_SIZE = 20

export const PDF_DOWNLOADABLE_STATUSES = new Set(['issued', 'paid', 'overdue'])

export const INVOICE_STATUS_LABELS: Record<string, string> = {
  draft: 'Draft',
  issued: 'Issued',
  paid: 'Paid',
  void: 'Void',
  overdue: 'Overdue',
}

export const INVOICE_STATUS_COLORS: Record<string, string> = {
  draft: 'secondary',
  issued: 'default',
  void: 'destructive',
}

export const INVOICE_SOURCE_LABELS: Record<string, string> = {
  admin_manual: 'Manual',
  user_application: 'Application',
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

export function getAvailableActions(status: string): InvoiceAction[] {
  return ACTION_RULES[status] ?? ['view']
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
