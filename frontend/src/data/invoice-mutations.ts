import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  createInvoice,
  updateInvoice,
  issueInvoice,
  voidInvoice,
  markPaid,
  upsertSellerConfig,
  applyInvoice,
} from '@/lib/api-generated'
import type { CreateInvoiceRequest, UpdateInvoiceRequest } from '@/lib/api-generated'
import type {
  InvoiceCreateFormData,
  InvoiceEditFormData,
  InvoiceSellerConfigFormData,
  ApplyInvoiceFormData,
} from '@/lib/schemas/invoice-forms'
import { displayPriceToCents } from '@/lib/invoice-utils'
import { getErrorMessage } from '@/lib/error-utils'
import { invoiceKeys } from '@/data/invoice-query-options'

export function useCreateInvoice(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (values: InvoiceCreateFormData) => {
      const body: CreateInvoiceRequest = {
        accountId: values.accountId,
        billingName: values.billingName,
        billingEmail: values.billingEmail ?? undefined,
        billingAddress: values.billingAddress ?? undefined,
        billingPhone: values.billingPhone ?? undefined,
        billingTaxId: values.billingTaxId,
        sellerName: values.sellerName,
        sellerEmail: values.sellerEmail ?? undefined,
        sellerAddress: values.sellerAddress ?? undefined,
        sellerPhone: values.sellerPhone ?? undefined,
        sellerTaxId: values.sellerTaxId,
        currency: values.currency,
        lineItems: values.lineItems.map((item) => ({
          name: item.name,
          description: item.description ?? undefined,
          quantity: item.quantity,
          unitPrice: displayPriceToCents(item.unitPrice),
        })),
        discountMode: values.discountMode ?? undefined,
        discountValue: values.discountValue ?? undefined,
        taxMode: values.taxMode ?? undefined,
        taxValue: values.taxValue ?? undefined,
        shippingMode: values.shippingMode ?? undefined,
        shippingValue: values.shippingValue ?? undefined,
        dueDate: values.dueDate || undefined,
        paymentTerms: values.paymentTerms ?? undefined,
        notes: values.notes ?? undefined,
        subscriptionId: values.subscriptionId ?? undefined,
        paymentAttemptId: values.paymentAttemptId ?? undefined,
      }
      const response = await createInvoice({ path: { realmId }, body })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Invoice created')
      queryClient.invalidateQueries({ queryKey: invoiceKeys.all(realmId) })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to create invoice: ${errorMessage}`)
    },
  })
}

export function useUpdateInvoice(realmId: string, invoiceId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (values: InvoiceEditFormData) => {
      const body: UpdateInvoiceRequest = {
        billingName: values.billingName,
        billingEmail: values.billingEmail ?? undefined,
        billingAddress: values.billingAddress ?? undefined,
        billingPhone: values.billingPhone ?? undefined,
        billingTaxId: values.billingTaxId,
        sellerName: values.sellerName,
        sellerEmail: values.sellerEmail ?? undefined,
        sellerAddress: values.sellerAddress ?? undefined,
        sellerPhone: values.sellerPhone ?? undefined,
        sellerTaxId: values.sellerTaxId,
        lineItems: values.lineItems.map((item) => ({
          name: item.name,
          description: item.description ?? undefined,
          quantity: item.quantity,
          unitPrice: displayPriceToCents(item.unitPrice),
        })),
        discountMode: values.discountMode ?? undefined,
        discountValue: values.discountValue ?? undefined,
        taxMode: values.taxMode ?? undefined,
        taxValue: values.taxValue ?? undefined,
        shippingMode: values.shippingMode ?? undefined,
        shippingValue: values.shippingValue ?? undefined,
        dueDate: values.dueDate ?? undefined,
        paymentTerms: values.paymentTerms ?? undefined,
        notes: values.notes ?? undefined,
      }
      const response = await updateInvoice({
        path: { realmId, invoiceId },
        body,
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Invoice updated')
      queryClient.invalidateQueries({ queryKey: invoiceKeys.all(realmId) })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to update invoice: ${errorMessage}`)
    },
  })
}

export function useIssueInvoice(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ invoiceId, issueDate }: { invoiceId: string; issueDate?: string }) => {
      const response = await issueInvoice({
        path: { realmId, invoiceId },
        body: { issueDate: issueDate ?? undefined },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Invoice issued')
      queryClient.invalidateQueries({ queryKey: invoiceKeys.all(realmId) })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to issue invoice: ${errorMessage}`)
    },
  })
}

export function useVoidInvoice(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ invoiceId, voidReason }: { invoiceId: string; voidReason?: string }) => {
      const response = await voidInvoice({
        path: { realmId, invoiceId },
        body: { voidReason: voidReason ?? undefined },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Invoice voided')
      queryClient.invalidateQueries({ queryKey: invoiceKeys.all(realmId) })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to void invoice: ${errorMessage}`)
    },
  })
}

export function useMarkPaid(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ invoiceId }: { invoiceId: string }) => {
      const response = await markPaid({
        path: { realmId, invoiceId },
        body: {},
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Invoice marked as paid')
      queryClient.invalidateQueries({ queryKey: invoiceKeys.all(realmId) })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to mark invoice as paid: ${errorMessage}`)
    },
  })
}

export function useUpsertSellerConfig(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (values: InvoiceSellerConfigFormData) => {
      const response = await upsertSellerConfig({
        path: { realmId },
        body: {
          sellerName: values.sellerName,
          sellerAddress: values.sellerAddress ?? undefined,
          sellerEmail: values.sellerEmail ?? undefined,
          sellerPhone: values.sellerPhone ?? undefined,
          sellerTaxId: values.sellerTaxId,
          defaultPaymentTerms: values.defaultPaymentTerms ?? undefined,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Seller config saved')
      queryClient.invalidateQueries({ queryKey: invoiceKeys.sellerConfig(realmId) })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to save seller config: ${errorMessage}`)
    },
  })
}

export function useApplyInvoice(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (values: ApplyInvoiceFormData) => {
      const response = await applyInvoice({
        path: { realmId },
        body: {
          currency: values.currency,
          paymentAttemptId: values.paymentAttemptId ?? undefined,
          subscriptionId: values.subscriptionId ?? undefined,
          billingName: values.billingName,
          billingEmail: values.billingEmail ?? undefined,
          billingAddress: values.billingAddress ?? undefined,
          billingPhone: values.billingPhone ?? undefined,
          notes: values.notes ?? undefined,
        },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Invoice application submitted')
      queryClient.invalidateQueries({ queryKey: invoiceKeys.myAll(realmId) })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to apply for invoice: ${errorMessage}`)
    },
  })
}
