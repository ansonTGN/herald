/**
 * Credit Note Webhook Helpers
 *
 * Builders and delivery helpers for Stripe credit note webhooks used in the
 * Credit Note demo/E2E tests.
 *
 * Uses `signStripeWebhook` from `webhook-renewal-simulation.ts` for signature
 * computation and implements an equivalent raw POST locally (the `postRaw`
 * helper in that file is internal and must not be imported).
 */

import type { APIRequestContext, APIResponse } from '@playwright/test'
import { signStripeWebhook } from '../../helpers/webhook-renewal-simulation'

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const API_TIMEOUT = 10_000

export interface StripeCreditNoteCreatedInput {
  /** Stripe event id (evt_*). */
  eventId: string
  /** Stripe credit note id (cn_*). */
  creditNoteId: string
  /** Stripe invoice id (in_*) this credit note refunds. */
  invoiceId: string
  /** Credit note total in smallest currency unit (cents). */
  total: number
  /** ISO 4217 currency code, e.g. "usd". */
  currency: string
}

export interface StripeCreditNoteVoidedInput {
  /** Stripe event id (evt_*). */
  eventId: string
  /** Stripe credit note id (cn_*). */
  creditNoteId: string
  /** Stripe invoice id (in_*) this credit note refunds. */
  invoiceId: string
  /** Credit note total in smallest currency unit (cents). */
  total: number
}

export interface StripeCreditNoteCreatedPayload {
  id: string
  type: 'credit_note.created'
  data: {
    object: {
      id: string
      invoice: string
      total: number
      currency: string
      [key: string]: unknown
    }
  }
}

export interface StripeCreditNoteVoidedPayload {
  id: string
  type: 'credit_note.voided'
  data: {
    object: {
      id: string
      invoice: string
      total: number
      [key: string]: unknown
    }
  }
}

export interface WebhookDeliveryResult {
  ok: boolean
  status: number
  body: string
  response: APIResponse
}

/**
 * Build a Stripe `credit_note.created` webhook payload.
 */
export function buildStripeCreditNoteCreatedPayload(
  input: StripeCreditNoteCreatedInput,
): StripeCreditNoteCreatedPayload {
  return {
    id: input.eventId,
    type: 'credit_note.created',
    data: {
      object: {
        id: input.creditNoteId,
        invoice: input.invoiceId,
        total: input.total,
        currency: input.currency,
      },
    },
  }
}

/**
 * Build a Stripe `credit_note.voided` webhook payload.
 */
export function buildStripeCreditNoteVoidedPayload(
  input: StripeCreditNoteVoidedInput,
): StripeCreditNoteVoidedPayload {
  return {
    id: input.eventId,
    type: 'credit_note.voided',
    data: {
      object: {
        id: input.creditNoteId,
        invoice: input.invoiceId,
        total: input.total,
      },
    },
  }
}

/**
 * Deliver a signed Stripe credit note webhook to the backend.
 *
 * Serializes the payload ONCE into a Buffer, signs it with `signStripeWebhook`,
 * and POSTs the raw bytes to `/api/third/pay/${realmId}/stripe/webhooks`.
 */
export async function deliverStripeCreditNoteWebhook(
  request: APIRequestContext,
  realmId: string,
  payload: StripeCreditNoteCreatedPayload | StripeCreditNoteVoidedPayload,
  webhookSecret?: string,
): Promise<WebhookDeliveryResult> {
  const rawBody = Buffer.from(JSON.stringify(payload), 'utf8')
  const signature = signStripeWebhook(rawBody, webhookSecret)

  const response = await request.post(`${BASE_URL}/api/third/pay/${realmId}/stripe/webhooks`, {
    headers: {
      'content-type': 'application/json',
      'stripe-signature': signature,
    },
    data: rawBody,
    timeout: API_TIMEOUT,
  })

  const body = await response.text().catch(() => '')
  return { ok: response.ok(), status: response.status(), body, response }
}
