import { useMutation, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  updateEntitlementMapping,
  syncProviderProducts,
  batchUpdateEntitlementMappings,
  createEntitlementMapping,
} from '@/lib/api-generated'
import type {
  UpdateEntitlementMappingRequest,
  BatchUpdateEntitlementMappingsRequest,
  BatchUpdateEntitlementMappingsResponse,
  CreateEntitlementMappingRequest,
} from '@/lib/api-generated'
import { getErrorMessage } from '@/lib/error-utils'
import { m } from '@/paraglide/messages'
import { queryKeys } from '@/data/query-options'

/**
 * Input shape for the legacy single-row PATCH hook below. The shared schema
 * was removed when the admin editor moved to a batch save; this local type
 * keeps the hook compilable for any remaining non-admin consumer.
 */
interface EntitlementMappingUpdateFormData {
  entitlementKey: string
  enabled: boolean
  pointsPerPeriod: number | null
  validityDays: number | null
  grantOnSubscribe: boolean
}

// ==================== Protected-price 409 detection ====================
//
// The batch endpoint rolls back the whole transaction and answers 409 with
// `{ code: "mapping_in_use", activeSubscriptions }` (typed as
// `MappingActiveSubscriptionLockErrorBody`) when a row transitions
// enabled true→false while protected by an active subscription. The mutation
// below follows the repo convention
// `if (response.error) throw response.error`, so these helpers receive the
// thrown `response.error` value — which for a 409 IS the typed lock body.
//
// NOTE: `MappingActiveSubscriptionLockErrorBody.code` is typed `string`
// (not the literal `'mapping_in_use'`), so `isProtectedPriceError` narrows
// with the literal check. If the backend ever renames the code, this check
// silently breaks — the 409 would then surface as a generic error toast.

/**
 * Error code the batch endpoint answers with on the active-subscription lock
 * (409). Extracted as a named constant so a backend rename surfaces here
 * rather than only as a silent magic-string match.
 */
export const PROTECTED_PRICE_ERROR_CODE = 'mapping_in_use' as const

/**
 * Returns true when the thrown error is the batch-save 409 lock body
 * (`{ code: 'mapping_in_use', activeSubscriptions }`). The caller should
 * then open the protected-price confirmation dialog instead of toasting.
 */
export function isProtectedPriceError(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  const e = error as { code?: unknown; activeSubscriptions?: unknown }
  return e.code === PROTECTED_PRICE_ERROR_CODE && typeof e.activeSubscriptions === 'number'
}

/**
 * Extracts the active-subscription count from a protected-price 409 error.
 * Returns `null` for any other shape (caller should fall back to a generic
 * message).
 */
export function extractActiveSubscriptions(error: unknown): number | null {
  if (!isProtectedPriceError(error)) return null
  return (error as { activeSubscriptions: number }).activeSubscriptions
}

// ==================== Cross-realm role 400 detection ====================
//
// When `grantedRoleIds` contains a role that does not belong to the target
// realm, the batch endpoint answers 400 with
// `{ code: 'role_not_in_realm', roleId, realmId }` (design §4.2.2). NOTE:
// `getErrorMessage` only reads `message`/`detail`/`error_description`/`error`
// — NOT `code` — so a plain toast would fall through to the generic fallback.
// This helper lets the mutation surface a friendlier, dedicated message
// instead, without widening `getErrorMessage`'s contract.

/**
 * Error code the batch endpoint answers with when a granted role is not in the
 * target realm (400). Extracted as a named constant so a backend rename
 * surfaces here rather than only as a silent magic-string match.
 */
export const ROLE_NOT_IN_REALM_ERROR_CODE = 'role_not_in_realm' as const

/**
 * Returns true when the thrown error is the batch-save 400 cross-realm role
 * body (`{ code: 'role_not_in_realm', roleId, realmId }`). The caller should
 * then surface a dedicated toast (the field is realm-scoped, so the only fix
 * is re-selecting roles that belong to this realm).
 */
export function isRoleNotInRealmError(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  const e = error as { code?: unknown }
  return e.code === ROLE_NOT_IN_REALM_ERROR_CODE
}

export function useUpdateEntitlementMapping(realmId: string, mappingId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (values: EntitlementMappingUpdateFormData) => {
      const body: UpdateEntitlementMappingRequest = {
        entitlementKey: values.entitlementKey,
        enabled: values.enabled,
        pointsPerPeriod: values.pointsPerPeriod ?? undefined,
        validityDays: values.validityDays ?? undefined,
        grantOnSubscribe: values.grantOnSubscribe,
        // bucketId is intentionally NOT sent: the PATCH handler preserves the
        // existing attribution (assignment is owned by the Credit Bucket
        // directory page). See entitlement-mapping-detail-dialog.tsx.
      }
      const response = await updateEntitlementMapping({
        path: { realmId, mappingId },
        body,
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      toast.success('Mapping updated')
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMappings(realmId, {}),
      })
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMapping(realmId, mappingId),
      })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to update mapping: ${errorMessage}`)
    },
  })
}

export function useSyncProviderProducts(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async ({ paymentProvider }: { paymentProvider: string }) => {
      const response = await syncProviderProducts({
        path: { realmId },
        body: { paymentProvider },
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: (data) => {
      if (data.syncStatus === 'completed') {
        toast.success(`Synced ${data.productsSynced} products and ${data.pricesSynced} prices`)
      } else if (data.syncStatus === 'partial') {
        toast.warning(`Partial sync: ${data.productsSynced} products synced, some prices failed`)
      }
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMappings(realmId, {}),
      })
    },
    onError: (error) => {
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to sync provider products: ${errorMessage}`)
    },
  })
}

// ==================== Batch Update (price-granularity) ====================

/**
 * Batch upsert of a product's price rows. The whole batch
 * is one server-side transaction; on 409 (`mapping_in_use`) the caller is
 * expected to catch the thrown `response.error` and surface the
 * protected-price confirmation dialog via `isProtectedPriceError` /
 * `extractActiveSubscriptions` — this hook intentionally does NOT toast on
 * a 409 so the page can handle the lock interactively.
 */
export function useBatchUpdateEntitlementMappings(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (body: BatchUpdateEntitlementMappingsRequest) => {
      const response = await batchUpdateEntitlementMappings({
        path: { realmId },
        body,
      })
      if (response.error) throw response.error
      return response.data as BatchUpdateEntitlementMappingsResponse
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMappings(realmId, {}),
      })
    },
    onError: (error) => {
      // A protected-price 409 is handled by the caller (confirmation dialog);
      // do not toast for it here.
      if (isProtectedPriceError(error)) return
      // A cross-realm role 400 (design §4.2.2) carries a `code` field that
      // `getErrorMessage` does not read; surface a dedicated, actionable toast
      // instead of the generic fallback. The only fix is re-selecting roles
      // that belong to this realm, so the message names the constraint.
      if (isRoleNotInRealmError(error)) {
        toast.error(m['billing.role_not_in_realm_error']())
        return
      }
      const errorMessage = getErrorMessage(error)
      toast.error(`Failed to save mappings: ${errorMessage}`)
    },
  })
}

// ==================== Create (single mapping) ====================
//
// `createEntitlementMapping` POSTs a single entitlement mapping. Two distinct
// server-side failure modes (design support-iap §4.2.2) MUST be surfaced
// differently in the dialog:
// - 409 Conflict: a row already exists for this
//   `(realm, provider, product, price)` tuple — i.e. a violation of
//   `uq_pem_realm_provider_product_price`. Actionable as "this product id
//   already exists"; the admin edits the inputs.
// - 23514 / non-4xx: a DB CHECK / internal-server defense (the backend maps a
//   Postgres CHECK failure to a 23514-style payload / 500). Surfaced as
//   "configuration error" and must NOT be confused with the 409 duplicate
//   branch. 401/403 are authz failures and fall through to the generic path.
//
// The thrown value is the generated client's `response.error` (an
// `ErrorResponse`-shaped object carrying `status`). These helpers narrow on
// that `status` field the same way `payment-providers-page.tsx` does
// (`error?.status === 409`); the hook does NOT toast for 409 or the
// 23514/non-4xx case so the dialog can branch (mirrors the
// `useBatchUpdateEntitlementMappings` "don't toast for the caller-handled
// error" precedent).

/**
 * Returns true when the thrown create-mapping error is the 409 duplicate
 * (`uq_pem_realm_provider_product_price` violation). The caller should surface
 * the `billing.create_mapping_duplicate` message.
 */
export function isCreateMappingDuplicateError(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  return (error as { status?: unknown }).status === 409
}

/**
 * Error code the backend surfaces verbatim when a create-mapping row fails a
 * DB CHECK constraint (the Postgres `check_violation` SQLSTATE 23514, mapped to
 * a body tag per design §4.2.2). Extracted as a named constant so a backend
 * rename surfaces here rather than only as a silent magic-string match.
 */
export const CREATE_MAPPING_CONFIG_ERROR_CODE = '23514' as const

/**
 * Returns true when the thrown create-mapping error is a DB CHECK / server
 * defense (design §4.2.2). The backend maps a Postgres CHECK failure (23514)
 * to either a 23514-tagged body or a non-4xx (e.g. 500). Surfaced as
 * `billing.create_mapping_config_error` and intentionally kept distinct from
 * the 409 duplicate branch. Authz failures (401/403) and the 400 validation
 * branch are NOT "config errors" — they fall through to the generic path.
 */
export function isCreateMappingConfigError(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  const e = error as { status?: unknown; code?: unknown }
  if (e.code === CREATE_MAPPING_CONFIG_ERROR_CODE) return true
  const status = e.status
  // Any non-4xx server-class failure is treated as a configuration error per
  // design §4.2.2 (covers the 500 mapping of a CHECK/DB defense).
  return typeof status === 'number' && status >= 500
}

/**
 * Create a single entitlement mapping (POST /entitlement-mappings). Used by the
 * "Create Mapping" dialog (FE-D03). The hook only invalidates the list on
 * success; 409 and 23514/non-4xx branches are intentionally NOT toasted here —
 * the dialog branches on them (see `isCreateMappingDuplicateError` /
 * `isCreateMappingConfigError`). Other errors fall through to a generic toast.
 */
export function useCreateEntitlementMapping(realmId: string) {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: async (body: CreateEntitlementMappingRequest) => {
      const response = await createEntitlementMapping({
        path: { realmId },
        body,
      })
      if (response.error) throw response.error
      return response.data
    },
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: queryKeys.entitlementMappings(realmId, {}),
      })
    },
    onError: (error) => {
      // 409 duplicate and 23514/non-4xx config errors are handled by the
      // dialog; do not toast for them here.
      if (isCreateMappingDuplicateError(error)) return
      if (isCreateMappingConfigError(error)) return
      const errorMessage = getErrorMessage(error)
      toast.error(`${m['billing.create_mapping_failed']()}: ${errorMessage}`)
    },
  })
}
