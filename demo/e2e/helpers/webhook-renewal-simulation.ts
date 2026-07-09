/**
 * Webhook Renewal Simulation Helper
 *
 * Sends signed synthetic webhook events to the billing webhook endpoints to
 * drive the subscription *renewal* write paths (Creem `subscription.paid`
 * renewal branch + Stripe `invoice.payment_succeeded` subscription renewal
 * branch) and the subscription *cancel/refund* role-revocation write paths
 * (Stripe `customer.subscription.deleted` + Stripe `charge.refunded` + Creem
 * `subscription.canceled`) without a real payment provider.
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
// Payload builders (cancel / refund — M4 role-revocation paths, DE-D02)
// ---------------------------------------------------------------------------

/**
 * Stripe `customer.subscription.deleted` payload.
 *
 * Drives the M4 subscription cancel/expire role-revocation chain
 * (US-PW-005 场景1). The backend dispatches on the literal event type at
 * `stripe_webhook_handlers.rs:3910` and parses via
 * `parse_subscription_deleted_payload` (`stripe_webhook_handlers.rs:596`,
 * struct at `:83`).
 *
 * BACKEND CONTRACT (load-bearing — do not regress):
 *  - `event.id` (top-level, string) required — else 400 `Missing event id`.
 *  - `event.type` (top-level, string) required — MUST equal the literal
 *    `"customer.subscription.deleted"`.
 *  - `data.object.id` (snake/native Stripe) required — else 400
 *    `Missing subscription id`. This is the EXTERNAL subscription id used by
 *    `find_by_external_subscription_id(..., "stripe")` to resolve the internal
 *    subscription UUID. For a demo-fulfilled subscription it equals the
 *    `provider_transaction_id` written at fulfillment time
 *    (`demo-fulfill-${attemptId}`).
 *  - `data.object.metadata.herald_user_id` (fallback `metadata.userId`) —
 *    UUID string REQUIRED, else 400 `Missing or invalid userId`.
 *  - `data.object.cancel_at_period_end` (bool, default false) — DRIVES THE
 *    CANCEL MODE. `false` → `ImmediateCancel` → `revoke_roles_by_payment_source`
 *    deletes the `source='payment'` rows for this subscription. `true` →
 *    `DefaultCancel` → NO revoke (period-end natural expiry). For the revoke
 *    demo, set `false`.
 *  - `data.object.metadata.herald_entitlement_key` (fallback
 *    `metadata.entitlementKey`) — optional.
 *  - `data.object.current_period_start` / `current_period_end` — optional unix
 *    i64 (only `current_period_end` is required downstream when
 *    `cancel_at_period_end=true`; with `false` it is optional).
 *
 * The parser does NOT read `customer` or `status`. Per design §5.5 /
 * US-PW-005, role revocation lands on the convergence point
 * `handle_subscription_cancel` (`subscription_service.rs:701`), `ImmediateCancel`
 * branch (`:733`) which calls `revoke_roles_by_payment_source(realm_id,
 * user_id, &subscription_id)` — deleting ONLY `user_roles` rows where
 * `source='payment' AND source_id == <subscription uuid>`. Manual grants
 * (source='manual') are untouched. `NotFound` is idempotent-success.
 */
export interface StripeSubscriptionDeletedPayload {
  /** Top-level Stripe event id (evt_*). Required by parse_event_id. */
  id: string
  type: 'customer.subscription.deleted'
  data: {
    object: {
      /** External subscription id (Stripe sub_*); resolves the internal sub. */
      id: string
      metadata: {
        /** UUID of the Herald user who holds the payment-granted role. */
        herald_user_id: string
        /** Optional entitlement key for targeted revocation. */
        herald_entitlement_key?: string
        [key: string]: unknown
      }
      /**
       * false → ImmediateCancel (revokes role). true → DefaultCancel (no revoke).
       * Default false per backend parser.
       */
      cancel_at_period_end: boolean
      [key: string]: unknown
    }
  }
}

export interface BuildStripeSubscriptionDeletedInput {
  /** Stripe event id (evt_*). Unique per delivery for clean idempotency. */
  eventId: string
  /**
   * External subscription id (Stripe sub_*) — MUST match the
   * `external_subscription_id` stored at fulfillment (the demo-fulfilled
   * subscription's `provider_transaction_id`). The backend resolves the
   * internal subscription UUID via `find_by_external_subscription_id`.
   */
  subscriptionId: string
  /** Herald user UUID holding the payment-granted role. Required. */
  userId: string
  /** Optional entitlement key for targeted revocation. */
  entitlementKey?: string
  /**
   * Drives cancel mode. false (default) → ImmediateCancel → revoke role.
   * true → DefaultCancel → no revoke. Tests asserting a revoke MUST pass false.
   */
  cancelAtPeriodEnd?: boolean
}

export function buildStripeSubscriptionDeletedPayload(
  input: BuildStripeSubscriptionDeletedInput,
): StripeSubscriptionDeletedPayload {
  return {
    id: input.eventId,
    type: 'customer.subscription.deleted',
    data: {
      object: {
        id: input.subscriptionId,
        metadata: {
          herald_user_id: input.userId,
          ...(input.entitlementKey !== undefined
            ? { herald_entitlement_key: input.entitlementKey }
            : {}),
        },
        cancel_at_period_end: input.cancelAtPeriodEnd ?? false,
      },
    },
  }
}

/**
 * Stripe `charge.refunded` payload.
 *
 * Drives the M4 refund role-revocation chain (US-PW-005 场景2). Backend
 * dispatches at `stripe_webhook_handlers.rs:3914`; parser
 * `parse_charge_refunded_payload` at `:626` (struct at `:93`).
 *
 * BACKEND CONTRACT (load-bearing — do not regress):
 *  - `event.id` + `event.type` top-level required; `event.type` MUST equal
 *    `"charge.refunded"`.
 *  - `data.object.id` (charge id) required — else 400 `Missing charge id`.
 *  - `data.object.amount` (i64) required — else 400 `Missing or invalid amount`.
 *  - `data.object.amount_refunded` (i64) required — else 400
 *    `Missing or invalid amount_refunded`.
 *  - `data.object.metadata.herald_user_id` (fallback `metadata.userId`) —
 *    UUID string required.
 *  - `data.object.metadata.herald_subscription_id` (fallback
 *    `metadata.subscriptionId`) — optional AT PARSE, but DOWNSTREAM REQUIRED
 *    to resolve the subscription for `refundType='subscription'`. The handler
 *    (`stripe_webhook_handlers.rs:2564`) resolves the subscription via
 *    `find_subscription_by_id(subscription_id)` — i.e. this is the INTERNAL
 *    subscription UUID (NOT the external id), resolved after fulfillment.
 *  - `data.object.metadata.refundType` — optional, default `"subscription"`.
 *    If `"topup"` it diverges to `revoke_topup_proportional` and does NOT
 *    revoke role. For the role-revoke demo, OMIT (defaults to subscription) or
 *    explicitly pass `"subscription"`.
 *
 * CONVERGENCE IS CONDITIONAL (design §5.5): default `refundType='subscription'`
 * + resolvable subscription → `handle_subscription_cancel(..., ImmediateCancel,
 * ...)` at `stripe_webhook_handlers.rs:2639` → `revoke_roles_by_payment_source`.
 * If the subscription cannot be resolved (no `herald_subscription_id`, or the
 * id does not match a row), the handler 400s `Cannot resolve bucket for
 * subscription refund`. Callers MUST pass the correct internal subscription UUID.
 *
 * The parser does NOT read `currency`, `payment_intent`, `customer`, or the
 * `refunds` array.
 */
export interface StripeChargeRefundedPayload {
  /** Top-level Stripe event id (evt_*). Required by parse_event_id. */
  id: string
  type: 'charge.refunded'
  data: {
    object: {
      /** Stripe charge id (ch_*). Required. */
      id: string
      /** Charge amount in minor currency units. Required. */
      amount: number
      /** Refunded amount in minor currency units. Required. */
      amount_refunded: number
      metadata: {
        /** Herald user UUID. Required. */
        herald_user_id: string
        /**
         * INTERNAL Herald subscription UUID (NOT the external Stripe sub id).
         * Required downstream for `refundType='subscription'` convergence.
         */
        herald_subscription_id: string
        /** "subscription" (default, revokes role) or "topup" (no role revoke). */
        refundType?: string
        [key: string]: unknown
      }
      [key: string]: unknown
    }
  }
}

export interface BuildStripeChargeRefundedInput {
  /** Stripe event id (evt_*). Unique per delivery for clean idempotency. */
  eventId: string
  /** Stripe charge id (ch_*). Required. */
  chargeId: string
  /** Charge amount in minor currency units. Required (must be > 0). */
  amount: number
  /** Refunded amount in minor currency units. Required (must be > 0). */
  amountRefunded: number
  /** Herald user UUID. Required. */
  userId: string
  /**
   * INTERNAL Herald subscription UUID (NOT the external Stripe sub id) —
   * resolved after fulfillment via the subscriptions API. Required for
   * `refundType='subscription'` convergence (else 400).
   */
  subscriptionId: string
  /**
   * "subscription" (default — revokes role via handle_subscription_cancel) or
   * "topup" (proportional topup revoke, no role revoke). For US-PW-005 场景2,
   * omit (defaults to subscription) or pass "subscription".
   */
  refundType?: string
}

export function buildStripeChargeRefundedPayload(
  input: BuildStripeChargeRefundedInput,
): StripeChargeRefundedPayload {
  if (input.amount <= 0) {
    throw new Error(
      `Stripe charge amount must be > 0 (got ${input.amount}). ` +
        'A zero/negative charge amount is not a valid refund source.',
    )
  }
  if (input.amountRefunded <= 0) {
    throw new Error(
      `Stripe amount_refunded must be > 0 (got ${input.amountRefunded}). ` +
        'A zero refund amount does not trigger the refund revoke path.',
    )
  }
  return {
    id: input.eventId,
    type: 'charge.refunded',
    data: {
      object: {
        id: input.chargeId,
        amount: input.amount,
        amount_refunded: input.amountRefunded,
        metadata: {
          herald_user_id: input.userId,
          herald_subscription_id: input.subscriptionId,
          ...(input.refundType !== undefined ? { refundType: input.refundType } : {}),
        },
      },
    },
  }
}

/**
 * Creem `subscription.canceled` payload.
 *
 * Drives the M4 cancel/expire role-revocation chain via Creem
 * (US-PW-005 场景1). Backend dispatches the literal `"subscription.canceled"`
 * → `handle_subscription_canceled` at `webhook_handlers.rs:1586` (routing at
 * `:2129`). Event-type extraction at `webhook_handlers.rs:2273` reads
 * `event.eventType` (CAMELCASE, not `event.type`) — missing → 400
 * `Missing eventType`. Parser `parse_subscription_canceled_payload` at `:563`
 * (struct `CreemSubscriptionCanceledPayload` at `:120`).
 *
 * BACKEND CONTRACT (load-bearing — do not regress):
 *  - Top-level `event.id` + `event.eventType` (camelCase) required. We ALSO
 *    emit `event.type` for parity with real Creem payloads (the dispatcher
 *    reads only `eventType`, so `type` is inert but harmless).
 *  - Object resolution quirk (`creem_event_object`, `:164`): reads
 *    `event.data.object` if non-null, else `event.object`. `data.object`
 *    preferred — this builder emits the object under `data.object`.
 *  - `object.cancelAtPeriodEnd` (bool, default false) — DRIVES CANCEL MODE.
 *    false → ImmediateCancel → revoke role; true → DefaultCancel → no revoke.
 *    For the revoke demo, set false.
 *  - `object.subscriptionId` (fallback `object.id`) required — else 400
 *    `Missing subscriptionId`. This is the EXTERNAL subscription id used by
 *    `find_by_external_subscription_id(..., "creem")` to resolve the internal
 *    UUID. For a demo-fulfilled subscription it equals the
 *    `provider_transaction_id`.
 *  - `object.productId` (fallback `object.product.id`) required — else 400
 *    `Missing productId`. We DUAL-WRITE `productId` (camelCase, primary) +
 *    `product` (string, parity) so both parse branches satisfy, mirroring the
 *    existing `buildCreemSubscriptionPaidRenewalPayload`.
 *  - `object.herald_user_id` (fallback chain: `object.metadata.herald_user_id`
 *    → `object.userId` → `object.metadata.userId`) — UUID string. Optional at
 *    parse; if absent the handler `:1598-1614` resolves via DB lookup by
 *    external_subscription_id. For demo determinism we INCLUDE
 *    `herald_user_id` in `object.metadata`.
 *  - `object.herald_entitlement_key` (fallback chain into metadata) — optional.
 *  - `object.currentPeriodStart` / `current_period_start`,
 *    `object.currentPeriodEnd` / `current_period_end` — optional; only
 *    `current_period_end` is required downstream when `cancelAtPeriodEnd=true`
 *    (with `false` it is optional).
 *  - STATUS FIELD QUIRK: the parser does NOT read `object.status`; it
 *    synthesizes status from `cancelAtPeriodEnd`. Do NOT rely on a `status`
 *    field. (The cancel parser also does NOT accept
 *    `current_period_start_date` / `current_period_end_date` — those are
 *    renewal-only.)
 *
 * Role revocation lands on `handle_subscription_cancel` (`:1648` ImmediateCancel
 * branch for Creem), same convergence point + `revoke_roles_by_payment_source`
 * semantics as the Stripe path. Manual grants untouched; `NotFound` idempotent.
 */
export interface CreemSubscriptionCanceledPayload {
  /** Top-level Creem event id (evt_*). Required by parse_event_id. */
  id: string
  /** Top-level event type; REQUIRED by handle_creem_webhook dispatcher (camelCase). */
  eventType: 'subscription.canceled'
  /** snake_case type — inert for the dispatcher (it reads `eventType`), kept for parity. */
  type: 'subscription.canceled'
  data: {
    object: {
      /** External subscription id (Creem sub_*); resolves the internal sub. Fallback for `object.subscriptionId`. */
      id: string
      /** camelCase subscription id — PRIMARY read path. */
      subscriptionId: string
      /** camelCase productId — PRIMARY read path of the backend parser. */
      productId: string
      /** snake_case product id — kept for parity (parser falls back to `object.product.id`). */
      product: string
      /** false → ImmediateCancel (revokes role). true → DefaultCancel (no revoke). Default false. */
      cancelAtPeriodEnd: boolean
      metadata: {
        /** Herald user UUID. Included for deterministic resolution (avoids DB-lookup fallback). */
        herald_user_id: string
        [key: string]: unknown
      }
      [key: string]: unknown
    }
  }
}

export interface BuildCreemSubscriptionCanceledInput {
  /** Top-level Creem event id (evt_*). Unique per delivery for clean idempotency. */
  eventId: string
  /**
   * External subscription id (Creem sub_*) — MUST match the
   * `external_subscription_id` stored at fulfillment. The backend resolves the
   * internal subscription UUID via `find_by_external_subscription_id`.
   */
  subscriptionId: string
  /** External Creem product id (prod_*). Required by parser. */
  productId: string
  /** Herald user UUID holding the payment-granted role. */
  userId: string
  /** Optional entitlement key for targeted revocation. */
  entitlementKey?: string
  /**
   * Drives cancel mode. false (default) → ImmediateCancel → revoke role.
   * true → DefaultCancel → no revoke. Tests asserting a revoke MUST pass false.
   */
  cancelAtPeriodEnd?: boolean
}

export function buildCreemSubscriptionCanceledPayload(
  input: BuildCreemSubscriptionCanceledInput,
): CreemSubscriptionCanceledPayload {
  return {
    id: input.eventId,
    eventType: 'subscription.canceled',
    type: 'subscription.canceled',
    data: {
      object: {
        id: input.subscriptionId,
        subscriptionId: input.subscriptionId,
        productId: input.productId,
        product: input.productId,
        cancelAtPeriodEnd: input.cancelAtPeriodEnd ?? false,
        metadata: {
          herald_user_id: input.userId,
          ...(input.entitlementKey !== undefined
            ? { herald_entitlement_key: input.entitlementKey }
            : {}),
        },
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

// ---------------------------------------------------------------------------
// Delivery (cancel / refund — DE-D02)
// ---------------------------------------------------------------------------

/**
 * Deliver a signed Stripe `customer.subscription.deleted` webhook.
 *
 * Drives the M4 cancel/expire role-revocation chain. See
 * `buildStripeSubscriptionDeletedPayload` for the backend contract.
 *
 * @returns the backend response (200 on success, 400 on signature / missing
 *   field / unresolvable-subscription issues). Callers SHOULD assert
 *   `result.ok` / `result.status` AND the downstream persistent state
 *   (permission/check `allowed=false`, user_roles source distinction) — NOT
 *   only the HTTP 200.
 */
export async function deliverStripeSubscriptionDeletedWebhook(
  request: APIRequestContext,
  realmId: string,
  payload: StripeSubscriptionDeletedPayload,
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

/**
 * Deliver a signed Stripe `charge.refunded` webhook.
 *
 * Drives the M4 refund role-revocation chain. See
 * `buildStripeChargeRefundedPayload` for the backend contract and the
 * CONDITIONAL convergence (refundType='subscription' + resolvable internal
 * subscription UUID).
 *
 * @returns the backend response (200 on success, 400 on signature / missing
 *   field / unresolvable-subscription issues). Callers SHOULD assert on
 *   persistent RBAC state, not only the HTTP status.
 */
export async function deliverStripeChargeRefundedWebhook(
  request: APIRequestContext,
  realmId: string,
  payload: StripeChargeRefundedPayload,
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

/**
 * Deliver a signed Creem `subscription.canceled` webhook.
 *
 * Drives the M4 cancel/expire role-revocation chain via Creem. See
 * `buildCreemSubscriptionCanceledPayload` for the backend contract.
 *
 * @returns the backend response (200 on success, 400 on signature / missing
 *   eventType / missing subscriptionId|productId issues). Callers SHOULD assert
 *   on persistent RBAC state, not only the HTTP status.
 */
export async function deliverCreemSubscriptionCanceledWebhook(
  request: APIRequestContext,
  realmId: string,
  payload: CreemSubscriptionCanceledPayload,
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

// Re-export the skew constant for callers that want to reason about
// timestamp freshness (DE-D03 live smoke may inject a stale timestamp to
// assert the server rejects it).
export { STRIPE_MAX_SKEW_SECONDS }
