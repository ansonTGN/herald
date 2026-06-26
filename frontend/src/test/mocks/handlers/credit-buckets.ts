import { http, HttpResponse } from 'msw'

const API_BASE_URL = 'http://localhost:3000'
const BASE = `${API_BASE_URL}/api/realms/:realmId/billing/credit-buckets`

export interface BucketInUseErrorBody {
  code: 'bucket_in_use'
  activeSubscriptions: number
  holdersWithBalance: number
}

/**
 * Per-domain MSW handlers for Credit Buckets.
 *
 * Default handlers are intentionally minimal; tests cover error branches
 * ad-hoc via the exported factory helpers below + `server.use(...)`.
 */

export const creditBucketsHandlers = [
  // Empty success list so list/detail queries used by the editor resolve.
  http.get(`${BASE}`, () =>
    HttpResponse.json({
      items: [],
      page: 0,
      pageSize: 20,
      total: 0,
    })
  ),
  http.get(`${BASE}/overview`, () => HttpResponse.json({ rows: [], grandTotal: {} })),
  // Client apps + entitlement mappings pulled by the editor multiselects
  // (`listClientApps` → /api/client/{realmId}; `listEntitlementMappings` →
  // /api/bill/{realmId}/entitlement-mappings). Empty lists are enough — the
  // editor's multiselects render from these and we don't exercise them here.
  http.get(`${API_BASE_URL}/api/client/:realmId`, () =>
    HttpResponse.json({ items: [], page: 0, pageSize: 100, total: 0 })
  ),
  http.get(`${API_BASE_URL}/api/bill/:realmId/entitlement-mappings`, () =>
    HttpResponse.json({ items: [], page: 0, pageSize: 100, total: 0 })
  ),
]

// ===== Error-scenario factories (consumed via server.use) =====

/** 409 `bucket_in_use` body. */
export function deleteBucketInUseHandler(body: BucketInUseErrorBody) {
  return http.delete(`${BASE}/:bucketId`, () => HttpResponse.json(body, { status: 409 }))
}

/** 409 `registration_pool_conflict` on PUT. */
export function updateBucketRegistrationConflictHandler() {
  return http.put(`${BASE}/:bucketId`, () =>
    HttpResponse.json({ code: 'registration_pool_conflict' }, { status: 409 })
  )
}

/** 409 `registration_pool_conflict` on POST (create flow). */
export function createBucketRegistrationConflictHandler() {
  return http.post(`${BASE}`, () =>
    HttpResponse.json({ code: 'registration_pool_conflict' }, { status: 409 })
  )
}
