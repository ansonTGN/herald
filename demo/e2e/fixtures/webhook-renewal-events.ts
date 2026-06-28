/**
 * Webhook Renewal Event Fixtures
 *
 * Reusable factories for synthetic renewal webhook payloads. Each call
 * generates FRESH `tran_` / `in_` / `sub_` / `evt_` identifiers so that
 * `provider_reference` idempotency keys (design §5.1) never collide across
 * tests or across renewal cycles within a single test:
 *
 *   - Creem  provider_reference = `creem_renewal:{sub}:{tran}`
 *   - Stripe provider_reference = `stripe_renewal:{sub}:{in_}`
 *
 * The factories are thin wrappers over the payload builders in
 * `helpers/webhook-renewal-simulation.ts`; they exist so DE-D02 scenario
 * tests can call one factory and get a fully-formed, signed-deliverable
 * payload without re-specifying boilerplate fields every time.
 *
 * The factories do NOT deliver the webhook — that is the helper's job.
 * The factories are pure functions and do not touch Playwright fixtures;
 * the CALLING test is responsible for going through the demo-page fixture
 * (per the demo-dev gate).
 */

import {
  buildCreemSubscriptionPaidRenewalPayload,
  buildStripePaymentSucceededPayload,
  type CreemSubscriptionPaidRenewalPayload,
  type StripeInvoicePaymentSucceededPayload,
} from '../helpers/webhook-renewal-simulation'

// ---------------------------------------------------------------------------
// ID generators
//
// `process.pid` + a monotonic counter keep ids unique within a single test
// worker run; `workers:1` (demo-fast) means there is no cross-worker risk.
// A coarse timestamp is folded in so ids also differ across runs, which
// matters for `payment_event.external_event_id` event-level idempotency.
// ---------------------------------------------------------------------------

const RUN_TAG = `${Date.now().toString(36)}-${process.pid.toString(36)}`
let tranSeq = 0
let invSeq = 0
let subSeq = 0
let evtSeq = 0
let cusSeq = 0
let piSeq = 0

function nextTranId(): string {
  tranSeq += 1
  return `tran_demo_${RUN_TAG}_${tranSeq}`
}

function nextInvoiceId(): string {
  invSeq += 1
  return `in_demo_${RUN_TAG}_${invSeq}`
}

function nextSubscriptionId(provider: 'creem' | 'stripe'): string {
  subSeq += 1
  const prefix = provider === 'creem' ? 'sub_creem_demo' : 'sub_stripe_demo'
  return `${prefix}_${RUN_TAG}_${subSeq}`
}

function nextEventId(): string {
  evtSeq += 1
  return `evt_demo_${RUN_TAG}_${evtSeq}`
}

function nextCustomerId(provider: 'creem' | 'stripe'): string {
  cusSeq += 1
  const prefix = provider === 'creem' ? 'cus_creem_demo' : 'cus_stripe_demo'
  return `${prefix}_${RUN_TAG}_${cusSeq}`
}

function nextPaymentIntentId(): string {
  piSeq += 1
  return `pi_demo_${RUN_TAG}_${piSeq}`
}

// ---------------------------------------------------------------------------
// Renewal scenario descriptors
// ---------------------------------------------------------------------------

/**
 * A fully-resolved Creem renewal scenario: the payload to deliver PLUS the
 * stable identifiers the test will later assert against (invoice list/detail,
 * admin UI). Tests should read `expectedExternalInvoiceId` /
 * `expectedSubscriptionId` from this object instead of re-deriving them.
 */
export interface CreemRenewalScenario {
  payload: CreemSubscriptionPaidRenewalPayload
  /** `last_transaction_id` — the external_invoice_id the backend will store. */
  expectedExternalInvoiceId: string
  /** Creem subscription id used in the event. */
  expectedSubscriptionId: string
  /** `provider_reference` the backend will derive: `creem_renewal:{sub}:{tran}`. */
  expectedProviderReference: string
  /** Amount in minor currency units (echoed for assertion convenience). */
  expectedAmount: number
  /** ISO 4217 currency code (echoed for assertion convenience). */
  expectedCurrency: string
}

export interface StripeRenewalScenario {
  payload: StripeInvoicePaymentSucceededPayload
  /** Invoice id (`in_`) — the external_invoice_id the backend will store. */
  expectedExternalInvoiceId: string
  /** Stripe subscription id used in the event. */
  expectedSubscriptionId: string
  /** `provider_reference` the backend will derive: `stripe_renewal:{sub}:{in_}`. */
  expectedProviderReference: string
  /** Amount in minor currency units (echoed for assertion convenience). */
  expectedAmount: number
  /** ISO 4217 currency code (echoed for assertion convenience). */
  expectedCurrency: string
}

// ---------------------------------------------------------------------------
// Shared option shape
// ---------------------------------------------------------------------------

export interface RenewalScenarioOptions {
  /**
   * Amount in minor currency units (e.g. cents). MUST be > 0 — zero-amount
   * cycles must NOT produce renewal invoices/attempts (design §5.1/§5.2/§5.3;
   * backend CHECK(amount > 0)). Defaults to 1000 (= $10.00 / 1000 minor).
   */
  amount?: number
  /** ISO 4217 currency. Defaults to "USD" (Creem) / "usd" (Stripe). */
  currency?: string
  /**
   * RFC3339 timestamp for the new period start. Defaults to "now". Used as
   * the external_invoice_id fallback when the transaction id is missing
   * (it is never missing for these fixtures, but the field is still required
   * by the backend parse chain).
   */
  currentPeriodStart?: string
}

function defaultAmount(opts: RenewalScenarioOptions | undefined): number {
  const amount = opts?.amount ?? 1000
  if (!Number.isInteger(amount) || amount <= 0) {
    throw new Error(
      `Renewal amount must be a positive integer (minor units); got ${amount}.`,
    )
  }
  return amount
}

// ---------------------------------------------------------------------------
// Factories
// ---------------------------------------------------------------------------

/**
 * Build a fresh Creem `subscription.paid` renewal scenario.
 *
 * Generates new `tran_`, `sub_`, `cus_` ids on every call so repeated
 * invocations (across tests or across renewal cycles) produce distinct
 * `provider_reference` idempotency keys.
 *
 * The returned payload intentionally OMITS `attempt_id` so the backend
 * routes it through the renewal branch (design §5.2 / assumption A2).
 */
export function makeCreemRenewalScenario(
  productId: string,
  options?: RenewalScenarioOptions,
): CreemRenewalScenario {
  const amount = defaultAmount(options)
  const currency = options?.currency ?? 'USD'
  const subscriptionId = nextSubscriptionId('creem')
  const transactionId = nextTranId()
  const customerId = nextCustomerId('creem')
  const currentPeriodStart = options?.currentPeriodStart ?? new Date().toISOString()

  const payload = buildCreemSubscriptionPaidRenewalPayload({
    subscriptionId,
    customerId,
    productId,
    amount,
    currency,
    lastTransactionId: transactionId,
    currentPeriodStart,
  })

  return {
    payload,
    expectedExternalInvoiceId: transactionId,
    expectedSubscriptionId: subscriptionId,
    expectedProviderReference: `creem_renewal:${subscriptionId}:${transactionId}`,
    expectedAmount: amount,
    expectedCurrency: currency,
  }
}

/**
 * Build a fresh Stripe `invoice.payment_succeeded` renewal scenario.
 *
 * Generates new `in_`, `sub_`, `cus_`, `pi_`, `evt_` ids on every call so
 * repeated invocations produce distinct `provider_reference` idempotency
 * keys and distinct event-level ids.
 *
 * The payload always carries `subscription` so the backend routes it through
 * the subscription renewal branch (design §5.3).
 */
export function makeStripeRenewalScenario(
  options?: RenewalScenarioOptions,
): StripeRenewalScenario {
  const amount = defaultAmount(options)
  const currency = options?.currency ?? 'usd'
  const invoiceId = nextInvoiceId()
  const subscriptionId = nextSubscriptionId('stripe')
  const customerId = nextCustomerId('stripe')
  const paymentIntentId = nextPaymentIntentId()
  const eventId = nextEventId()

  const payload = buildStripePaymentSucceededPayload({
    eventId,
    invoiceId,
    subscriptionId,
    customerId,
    paymentIntentId,
    total: amount,
    currency,
  })

  return {
    payload,
    expectedExternalInvoiceId: invoiceId,
    expectedSubscriptionId: subscriptionId,
    expectedProviderReference: `stripe_renewal:${subscriptionId}:${invoiceId}`,
    expectedAmount: amount,
    expectedCurrency: currency,
  }
}

// ---------------------------------------------------------------------------
// Reset (test isolation helper)
//
// Not required for correctness (ids are run-scoped), but exposed so a test
// suite that wants deterministic sequence numbers within a run can reset
// between describe blocks.
// ---------------------------------------------------------------------------

export function __resetRenewalEventSequences(): void {
  tranSeq = 0
  invSeq = 0
  subSeq = 0
  evtSeq = 0
  cusSeq = 0
  piSeq = 0
}
