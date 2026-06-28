/**
 * Webhook Renewal Simulation Helper
 *
 * Sends signed synthetic webhook events to the billing webhook endpoints to
 * drive the subscription *renewal* write paths (Creem `subscription.paid`
 * renewal branch + Stripe `invoice.payment_succeeded` subscription renewal
 * branch) without a real payment provider.
 *
 * -------------------------------------------------------------------------
 * SOURCE OF TRUTH — backend signature verification:
 *
 * - Creem: `backend/api-billing/src/webhooks.rs:22-42`
 *   `verify_webhook_signature(payload, signature_header, webhook_secret)`:
 *     * HMAC-SHA256 over the RAW request body bytes (not a stringified copy).
 *     * Header name: `creem-signature`.
 *     * Header value: PURE HEX of the HMAC digest. NO `t=` prefix, NO comma.
 *     * `hex::decode(signature_header)` then `mac.verify_slice(...)`.
 *
 * - Stripe: `backend/infra-stripe/src/client.rs:578-648`
 *   `StripeClient::verify_webhook_signature(payload, signature, secret)`:
 *     * Header name: `stripe-signature`.
 *     * Header value format: `t={unix_seconds},v1={hex_hmac}`.
 *     * Signed payload = `"{timestamp}.{raw_body_string}"` (UTF-8 lossy).
 *     * HMAC-SHA256 keyed by the webhook secret, hex-encoded as `v1`.
 *     * Replay window: timestamp must be within ±900s of server now.
 *
 * - Routes (`backend/api-billing/src/routes.rs:62-78`):
 *     * Creem:  POST /api/third/pay/{realmId}/creem/webhooks  → handle_creem_webhook
 *     * Stripe: POST /api/third/pay/{realmId}/stripe/webhooks → handle_stripe_webhook
 *
 * - Webhook secret source: per-realm config seeded via
 *   `seedCreemConfig` / `seedStripeConfig` (`demo/e2e/secrets/realm-seed.ts`).
 *   This helper reads `CREEM_WEBHOOK_SECRET` / `STRIPE_WEBHOOK_SECRET` from
 *   `process.env` (loaded from `demo/.env.demo`); the seeded per-realm value
 *   MUST match this env value or the backend returns 400.
 * -------------------------------------------------------------------------
 *
 * BYTE-CONSISTENCY CAVEAT (load-bearing):
 * The signature is computed over the RAW body bytes that hit the wire. The
 * `deliver*` helpers therefore serialize the payload ONCE with
 * `JSON.stringify` and pass the resulting UTF-8 Buffer as Playwright's
 * `request.post(..., { data: Buffer })`. Passing a Buffer (not an object)
 * prevents Playwright from re-serializing the body and invalidating the
 * signature. Callers MUST NOT swap `data: rawBody` for `data: payload`.
 */

import { createHmac, timingSafeEqual, randomUUID } from 'crypto'
import { type APIRequestContext, type APIResponse } from '@playwright/test'

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'
const API_TIMEOUT = 10_000

/** Stripe replay-window enforced server-side (client.rs:616-628) is ±900s. */
const STRIPE_MAX_SKEW_SECONDS = 900

/**
 * Webhook endpoint path templates.
 * Kept in sync with `backend/api-billing/src/routes.rs:65-72`.
 * Do NOT inline these at call sites.
 */
export const WEBHOOK_ROUTES = {
  creem: (realmId: string) => `/api/third/pay/${realmId}/creem/webhooks`,
  stripe: (realmId: string) => `/api/third/pay/${realmId}/stripe/webhooks`,
} as const

/** Header names used by the backend verifiers. */
export const WEBHOOK_HEADERS = {
  creem: 'creem-signature',
  stripe: 'stripe-signature',
} as const

// ---------------------------------------------------------------------------
// Secret accessors (fail loud — Rule 12)
// ---------------------------------------------------------------------------

function getCreemWebhookSecret(): string {
  const secret = process.env.CREEM_WEBHOOK_SECRET?.trim()
  if (!secret) {
    throw new Error(
      'CREEM_WEBHOOK_SECRET is required to sign Creem renewal webhooks. ' +
        'Set it in demo/.env.demo (must match the value seeded into the realm ' +
        'via seedCreemConfig).',
    )
  }
  return secret
}

function getStripeWebhookSecret(): string {
  const secret = process.env.STRIPE_WEBHOOK_SECRET?.trim()
  if (!secret) {
    throw new Error(
      'STRIPE_WEBHOOK_SECRET is required to sign Stripe renewal webhooks. ' +
        'Set it in demo/.env.demo (must match the value seeded into the realm ' +
        'via seedStripeConfig).',
    )
  }
  return secret
}

// ---------------------------------------------------------------------------
// Signature primitives
// ---------------------------------------------------------------------------

/**
 * Compute the Creem webhook signature header value for a raw body.
 *
 * Returns the PURE HEX-encoded HMAC-SHA256 digest (NO `t=` prefix),
 * matching `verify_webhook_signature` in `webhooks.rs:22-42`.
 */
export function signCreemWebhook(rawBody: Buffer | string, secret?: string): string {
  const key = secret ?? getCreemWebhookSecret()
  const buf = Buffer.isBuffer(rawBody) ? rawBody : Buffer.from(rawBody, 'utf8')
  return createHmac('sha256', key).update(buf).digest('hex')
}

/**
 * Compute the Stripe webhook signature header value for a raw body.
 *
 * Returns `t={unixSeconds},v1={hexHmac}` where the HMAC is taken over the
 * signed payload `"{timestamp}.{rawBody}"`, matching
 * `StripeClient::verify_webhook_signature` in `client.rs:578-648`.
 *
 * @param timestamp Unix seconds used as `t=`. Defaults to now. Stripe's
 *   server-side replay window is ±900s; tests rarely need to override this.
 */
export function signStripeWebhook(
  rawBody: Buffer | string,
  secret?: string,
  timestamp: number = Math.floor(Date.now() / 1000),
): string {
  const key = secret ?? getStripeWebhookSecret()
  const bodyStr = Buffer.isBuffer(rawBody) ? rawBody.toString('utf8') : rawBody
  // Mirror backend: `format!("{}.{}", timestamp, String::from_utf8_lossy(payload))`.
  const signedPayload = `${timestamp}.${bodyStr}`
  const v1 = createHmac('sha256', key).update(signedPayload, 'utf8').digest('hex')
  return `t=${timestamp},v1=${v1}`
}

/**
 * Constant-time equality helper, mirroring the backend's constant-time
 * comparison. Exported so DE-D03 (live smoke) can self-verify a freshly
 * computed signature against an expected value without pulling in a third
 * party lib.
 */
export function safeEqualHex(a: string, b: string): boolean {
  const bufA = Buffer.from(a, 'hex')
  const bufB = Buffer.from(b, 'hex')
  if (bufA.length !== bufB.length) return false
  return timingSafeEqual(bufA, bufB)
}

// ---------------------------------------------------------------------------
// Payload builders (renewal-specific)
// ---------------------------------------------------------------------------

/**
 * Creem `subscription.paid` — RENEWAL branch payload.
 *
 * Per design §5.2 the renewal branch is selected by the ABSENCE of
 * `attempt_id` (first-payment events carry `attempt_id` from checkout
 * metadata and short-circuit before invoice logic). Required fields for
 * the renewal invoice write:
 *   - object.id                       → ext_sub_id (subscription being renewed)
 *   - object.last_transaction_id      → tran_, used as external_invoice_id
 *   - object.amount                   → minor currency units (MUST be > 0)
 *   - object.currency                 → ISO 4217
 *   - object.current_period_start     → RFC3339, fallback invoice id comp
 *   - object.customer / object.product / object.status etc → parse chain
 *
 * `attempt_id` is intentionally OMITTED.
 *
 * BACKEND CONTRACT (load-bearing — do not regress):
 * `handle_creem_webhook` (webhook_handlers.rs:2259-2263) requires the TOP-LEVEL
 * `event.id` and `event.eventType`, else 400. The object parser
 * (parse_subscription_paid_payload, webhook_handlers.rs:463-472) reads
 * `object["productId"]` (camelCase) FIRST, falling back to
 * `object["product"]["id"]` — a bare `object.product` STRING does NOT satisfy
 * either branch and yields `400 Missing productId`. This builder therefore
 * emits BOTH `object.productId` (camelCase, primary) and `object.product`
 * (snake, kept for audit/real-Creem parity), plus the top-level `id` +
 * `eventType` the dispatcher demands. Callers no longer need to augment.
 */
export interface CreemSubscriptionPaidRenewalPayload {
  /** Top-level event id (Creem evt_*); required by parse_event_id. */
  id: string
  /** Top-level event type; required by handle_creem_webhook dispatcher. */
  eventType: 'subscription.paid'
  type: 'subscription.paid'
  object: {
    id: string
    customer: string
    /** camelCase productId — PRIMARY read path of the backend parser. */
    productId: string
    /** snake_case product id — kept for parity with real Creem payloads. */
    product: string
    status: string
    amount: number
    currency: string
    last_transaction_id: string
    current_period_start: string
    current_period_end?: string
    [key: string]: unknown
  }
}

export interface BuildCreemSubscriptionPaidRenewalInput {
  /** External subscription id (Creem sub_*). */
  subscriptionId: string
  /** External customer id (Creem cus_*). */
  customerId: string
  /** External product id (Creem prod_*). */
  productId: string
  /** Renewal amount in minor currency units (e.g. cents). MUST be > 0. */
  amount: number
  /** ISO 4217 currency code, e.g. "USD". */
  currency: string
  /** Creem transaction id (tran_*); becomes external_invoice_id. */
  lastTransactionId: string
  /** RFC3339 timestamp of the new period start. */
  currentPeriodStart: string
  currentPeriodEnd?: string
  /** Subscription status reported by Creem. Default "active". */
  status?: string
  /**
   * Top-level Creem event id (evt_*). Required by the backend dispatcher.
   * Callers SHOULD pass a unique value per delivery for clean idempotency;
   * defaults to a generated evt_* when omitted.
   */
  eventId?: string
}

export function buildCreemSubscriptionPaidRenewalPayload(
  input: BuildCreemSubscriptionPaidRenewalInput,
): CreemSubscriptionPaidRenewalPayload {
  if (input.amount <= 0) {
    throw new Error(
      `Creem renewal amount must be > 0 (got ${input.amount}). ` +
        'Zero-amount cycles must NOT generate renewal invoices/attempts ' +
        '(design §5.2; backend CHECK(amount > 0)).',
    )
  }
  return {
    id: input.eventId ?? `evt_${randomUUID()}`,
    eventType: 'subscription.paid',
    type: 'subscription.paid',
    object: {
      id: input.subscriptionId,
      customer: input.customerId,
      productId: input.productId,
      product: input.productId,
      status: input.status ?? 'active',
      amount: input.amount,
      currency: input.currency,
      last_transaction_id: input.lastTransactionId,
      current_period_start: input.currentPeriodStart,
      ...(input.currentPeriodEnd !== undefined
        ? { current_period_end: input.currentPeriodEnd }
        : {}),
    },
  }
}

/**
 * Stripe `invoice.payment_succeeded` — subscription renewal payload.
 *
 * Per design §5.3 the renewal branch requires `data.object.subscription` to
 * be present (one-time invoices have no subscription). Required fields:
 *   - data.object.id             → in_, used as external_invoice_id
 *   - data.object.subscription   → sub_, used in provider_reference + invoice sub link
 *   - data.object.total          → minor currency units (MUST be > 0)
 *   - data.object.currency       → ISO 4217
 *   - data.object.customer       → cus_, parse chain
 *   - data.object.payment_intent → pi_, becomes external_order_id
 */
export interface StripeInvoicePaymentSucceededPayload {
  id: string
  type: 'invoice.payment_succeeded'
  data: {
    object: {
      id: string
      subscription: string
      customer: string
      payment_intent: string
      total: number
      currency: string
      status?: string
      [key: string]: unknown
    }
  }
}

export interface BuildStripePaymentSucceededInput {
  /** Stripe event id (evt_*). Used for event-level idempotency on the backend. */
  eventId: string
  /** Stripe invoice id (in_*); becomes external_invoice_id. */
  invoiceId: string
  /** Stripe subscription id (sub_*); required for the renewal branch. */
  subscriptionId: string
  /** Stripe customer id (cus_*). */
  customerId: string
  /** Stripe payment intent id (pi_*); becomes external_order_id. */
  paymentIntentId: string
  /** Invoice total in minor currency units. MUST be > 0. */
  total: number
  /** ISO 4217 currency code, e.g. "usd". */
  currency: string
  /** Optional invoice status. Defaults to "paid". */
  status?: string
}

export function buildStripePaymentSucceededPayload(
  input: BuildStripePaymentSucceededInput,
): StripeInvoicePaymentSucceededPayload {
  if (input.total <= 0) {
    throw new Error(
      `Stripe invoice total must be > 0 (got ${input.total}). ` +
        'Zero-amount invoices must NOT generate renewal invoices/attempts ' +
        '(design §5.3; backend CHECK(amount > 0)).',
    )
  }
  return {
    id: input.eventId,
    type: 'invoice.payment_succeeded',
    data: {
      object: {
        id: input.invoiceId,
        subscription: input.subscriptionId,
        customer: input.customerId,
        payment_intent: input.paymentIntentId,
        total: input.total,
        currency: input.currency,
        status: input.status ?? 'paid',
      },
    },
  }
}

// ---------------------------------------------------------------------------
// Delivery
// ---------------------------------------------------------------------------

/**
 * Outcome of a webhook delivery. `ok` mirrors `response.ok()` (2xx).
 * Signature failures surface as HTTP 400 from the backend — callers SHOULD
 * assert on `status` / `ok` rather than only on `ok`.
 */
export interface WebhookDeliveryResult {
  ok: boolean
  status: number
  body: string
  response: APIResponse
}

async function postRaw(
  request: APIRequestContext,
  path: string,
  signatureHeaderName: string,
  signatureHeaderValue: string,
  rawBody: Buffer,
): Promise<WebhookDeliveryResult> {
  const response = await request.post(`${BASE_URL}${path}`, {
    headers: {
      // The backend reads the raw body off the wire; we set application/json
      // for routing/audit but the body bytes are exactly `rawBody`.
      'content-type': 'application/json',
      [signatureHeaderName]: signatureHeaderValue,
    },
    // CRITICAL: pass a Buffer so Playwright does NOT re-serialize the body.
    // See BYTE-CONSISTENCY CAVEAT in the file header.
    data: rawBody,
    timeout: API_TIMEOUT,
  })
  const body = await response.text().catch(() => '')
  return { ok: response.ok(), status: response.status(), body, response }
}

/**
 * Deliver a signed Creem `subscription.paid` renewal webhook.
 *
 * @returns the backend response (200 on success, 400 on signature/idempotency
 *   issues). Callers SHOULD assert `result.ok` / `result.status`.
 */
export async function deliverCreemRenewalWebhook(
  request: APIRequestContext,
  realmId: string,
  payload: CreemSubscriptionPaidRenewalPayload,
): Promise<WebhookDeliveryResult> {
  const rawBody = Buffer.from(JSON.stringify(payload), 'utf8')
  const signature = signCreemWebhook(rawBody)
  return postRaw(
    request,
    WEBHOOK_ROUTES.creem(realmId),
    WEBHOOK_HEADERS.creem,
    signature,
    rawBody,
  )
}

/**
 * Deliver a signed Stripe `invoice.payment_succeeded` renewal webhook.
 *
 * @returns the backend response (200 on success, 400 on signature/idempotency
 *   issues). Callers SHOULD assert `result.ok` / `result.status`.
 */
export async function deliverStripeRenewalWebhook(
  request: APIRequestContext,
  realmId: string,
  payload: StripeInvoicePaymentSucceededPayload,
): Promise<WebhookDeliveryResult> {
  const rawBody = Buffer.from(JSON.stringify(payload), 'utf8')
  const signature = signStripeWebhook(rawBody)
  return postRaw(
    request,
    WEBHOOK_ROUTES.stripe(realmId),
    WEBHOOK_HEADERS.stripe,
    signature,
    rawBody,
  )
}

// Re-export the skew constant for callers that want to reason about
// timestamp freshness (DE-D03 live smoke may inject a stale timestamp to
// assert the server rejects it).
export { STRIPE_MAX_SKEW_SECONDS }
