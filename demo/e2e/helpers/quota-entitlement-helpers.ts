/**
 * Quota Entitlement Internal Helpers (demo/test-only)
 *
 * Drives the internal `/api/internal/points/{realmId}/quota-entitlement/*`
 * endpoints to construct a `PointsQuotaEntitlement` directly, replicating what
 * the Stripe webhook path (`handle_subscription_paid` →
 * `subscription_service.grant_quota_entitlement`) would produce — without
 * driving a real Stripe checkout.
 *
 * Why this exists: seeded Stripe price IDs are placeholders (DE-D01 §27) and
 * must never be driven through real Stripe, so fast demo E2E tests cannot use
 * the purchase flow to obtain a window-quota entitlement. These helpers let the
 * dashboard UI tests seed quota directly.
 *
 * Auth model: shared `X-Internal-API-Key` header / `INTERNAL_API_KEY` env var,
 * identical to `payment-simulation.ts` (the runner injects
 * `INTERNAL_API_KEY=demo-internal-api-key`).
 */

import { type APIRequestContext } from '@playwright/test'

import type { QuotaWindowFixture } from '../fixtures/points-quota.fixtures'

const API_TIMEOUT = 10000
const BASE_URL = process.env.BASE_URL || 'http://localhost:3000'

function getInternalApiKey(): string {
  const apiKey = process.env.INTERNAL_API_KEY?.trim()
  if (!apiKey) {
    throw new Error('INTERNAL_API_KEY is required for quota entitlement helpers')
  }
  return apiKey
}

export interface QuotaEntitlementResult {
  success: boolean
  entitlementId?: string
  status?: string
  error?: string
}

export interface GrantQuotaEntitlementOptions {
  userId: string
  bucketId: string
  /** Stable anchor identifying this grant. Revoke targets the same value. */
  sourceId: string
  windows: QuotaWindowFixture[]
  /** `subscription_credit` (default) or `free_periodic_credit`. */
  creditType?: string
  /** `subscription_initial` (default). */
  sourceType?: string
  /** RFC3339 timestamp; omit for an entitlement that never expires. */
  effectiveUntil?: string
}

/**
 * Grant (or idempotently re-grant) a window-quota entitlement.
 *
 * The backend derives the idempotency key from `sourceId`, so a replayed grant
 * converges on the same row. Pair with `revokeQuotaEntitlement` using the same
 * `sourceId` to obtain a clean baseline between tests.
 */
export async function grantQuotaEntitlement(
  request: APIRequestContext,
  realmId: string,
  options: GrantQuotaEntitlementOptions,
): Promise<QuotaEntitlementResult> {
  try {
    const response = await request.post(
      `${BASE_URL}/api/internal/points/${realmId}/quota-entitlement/grant`,
      {
        headers: {
          'Content-Type': 'application/json',
          'X-Internal-API-Key': getInternalApiKey(),
        },
        data: {
          userId: options.userId,
          bucketId: options.bucketId,
          sourceId: options.sourceId,
          creditType: options.creditType,
          sourceType: options.sourceType,
          effectiveUntil: options.effectiveUntil,
          windows: options.windows.map((w) => ({
            key: w.key,
            windowSeconds: w.windowSeconds,
            limit: w.limit,
          })),
        },
        timeout: API_TIMEOUT,
      },
    )

    if (response.ok()) {
      const data = await response.json()
      return {
        success: true,
        entitlementId: data.entitlementId,
        status: data.status,
      }
    }

    const error = await response.text()
    return {
      success: false,
      error: `grant failed: ${response.status()} - ${error}`,
    }
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    }
  }
}

export interface RevokeQuotaEntitlementOptions {
  userId: string
  bucketId: string
  sourceId: string
  /** `subscription_credit` (default) or `free_periodic_credit`. */
  creditType?: string
}

/**
 * Revoke the active quota entitlement identified by `sourceId`. Idempotent: a
 * no-match returns `{ success: true }` (the post-condition "no active
 * entitlement" holds).
 */
export async function revokeQuotaEntitlement(
  request: APIRequestContext,
  realmId: string,
  options: RevokeQuotaEntitlementOptions,
): Promise<QuotaEntitlementResult> {
  try {
    const response = await request.post(
      `${BASE_URL}/api/internal/points/${realmId}/quota-entitlement/revoke`,
      {
        headers: {
          'Content-Type': 'application/json',
          'X-Internal-API-Key': getInternalApiKey(),
        },
        data: {
          userId: options.userId,
          bucketId: options.bucketId,
          sourceId: options.sourceId,
          creditType: options.creditType,
        },
        timeout: API_TIMEOUT,
      },
    )

    if (response.ok()) {
      return { success: true }
    }

    const error = await response.text()
    return {
      success: false,
      error: `revoke failed: ${response.status()} - ${error}`,
    }
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    }
  }
}
