// Points Wallet Handlers

use std::collections::BTreeMap;

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use uuid::Uuid;

use crate::types::{
    BalancesByType, ListWalletsByBucketResponse, ListWalletsQuery, PointsWalletResponse,
    WalletByBucketResponse,
};
use herald_api_base::application::http::auth::util::require_permission;
use herald_api_base::application::http::common::auth_utils::require_authenticated_user_in_realm;
use herald_api_base::application::http::common::error_codes::POINTS_UNIT;
use herald_api_base::application::http::server::api_entities::{
    ApiError, ApiResult, ErrorResponse,
};
use herald_api_base::application::http::state::AppState;
use herald_core::domain::authentication::Identity;
use herald_core::domain::points::entities::{CreditType, PointsWallet};
use herald_core::domain::points::ports::{PointsRepository, WalletFilters};

/// Map a derived `(CreditType, amount)` slice into the 5-field
/// `BalancesByType`. Unknown credit types are ignored (none
/// exist today; future types would need an explicit field added first).
fn derived_to_balances_by_type(derived: &[(CreditType, i64)]) -> BalancesByType {
    let mut out = BalancesByType::default();
    for (credit_type, amount) in derived {
        match credit_type {
            CreditType::TopupCredit => out.topup = out.topup.saturating_add(*amount),
            CreditType::SubscriptionCredit => {
                out.subscription = out.subscription.saturating_add(*amount)
            }
            CreditType::GrantedCredit => out.granted = out.granted.saturating_add(*amount),
            CreditType::RegistrationCredit => {
                out.registration = out.registration.saturating_add(*amount)
            }
            CreditType::FreePeriodicCredit => {
                out.free_periodic = out.free_periodic.saturating_add(*amount)
            }
        }
    }
    out
}

/// Map a domain `PointsWallet` + derived balances to the HTTP response shape.
/// The 5 typed balances and `total_balance` come from the
/// derived SUM (same predicate as consumption — future-effective rows are
/// excluded, so the user-visible balance never leaks un-granted periods);
/// analytics (`total_recharged` / `total_consumed`) stay on the Stored wallet
/// projection (lifetime totals, unaffected by `effective_at`).
///
/// The aggregate user-total view (`find_by_user_id` for a multi-bucket user)
/// has `bucket_id = None` and no single concrete wallet to expose — return
/// `id: None` so a client can never mistake a synthesized id for "the wallet"
/// (review #6 chimera fix). For a single-bucket user `bucket_id` is `Some`
/// and `id` is that wallet row's id.
fn wallet_to_response(
    account: PointsWallet,
    derived: Vec<(CreditType, i64)>,
) -> PointsWalletResponse {
    let bucket_id = account.bucket_id;
    let total_balance = derived_to_balances_by_type(&derived).total();
    PointsWalletResponse {
        // Only expose a concrete wallet id when the row is tied to a single
        // bucket. The aggregate view (bucket_id = None) has no canonical id.
        id: bucket_id.map(|_| account.id),
        user_id: account.user_id,
        realm_id: account.realm_id,
        bucket_id,
        balance: total_balance,
        total_paid_granted: account.total_recharged,
        total_recharged: account.total_recharged,
        total_consumed: account.total_consumed,
        status: account.status.as_str().to_string(),
        created_at: account.created_at.to_rfc3339(),
        updated_at: account.updated_at.to_rfc3339(),
        unit: POINTS_UNIT.to_string(),
        currency: POINTS_UNIT.to_string(),
    }
}

/// Directory metadata for a Credit Bucket, used to enrich `WalletByBucket`
/// rows with `name` / `enabled`. Looked up once per request
/// from `billing_repository.list_credit_buckets` to avoid N+1.
struct BucketDirInfo {
    name: String,
    enabled: bool,
}

/// Fetch the realm's Credit Bucket directory once and index it by id.
///
/// Used to populate `WalletByBucketResponse.name` / `enabled` without an N+1
/// per-row lookup. Errors are logged and swallowed: the wallets view is still
/// valid without directory enrichment, and a directory outage should not 500
/// the balance page.
async fn load_bucket_dir(state: &AppState, realm_id: &str) -> BTreeMap<Uuid, BucketDirInfo> {
    match state.billing_repository.list_credit_buckets(realm_id).await {
        Ok(items) => items
            .into_iter()
            .map(|item| {
                let b = item.bucket;
                (
                    b.id,
                    BucketDirInfo {
                        name: b.name,
                        enabled: b.enabled,
                    },
                )
            })
            .collect(),
        Err(e) => {
            tracing::warn!(
                realm_id = %realm_id,
                error = %e,
                "Failed to load credit bucket directory for wallet enrichment; \
                 name/enabled will be unset"
            );
            BTreeMap::new()
        }
    }
}

/// Group the per-wallet rows by `(bucket_id, user_id)` and produce the
/// `WalletByBucket[]` aggregation.
///
/// The 5 typed balances per `(user, bucket)` come from the derived SUM (same
/// predicate as consumption — future-effective pre-grant rows are excluded so
/// the admin view never leaks un-granted periods). The repository read happens
/// once per distinct `(user, bucket)` pair on the page: pages are bounded
/// (≤100 rows, typically far fewer distinct pairs), keeping this off the N+1
/// cliff. `bucket_dir` carries `(name, enabled)` per bucket id resolved once
/// from the bucket directory.
async fn group_wallets_by_bucket(
    state: &AppState,
    realm_id: &str,
    wallets: Vec<PointsWallet>,
    bucket_dir: &BTreeMap<Uuid, BucketDirInfo>,
) -> (Vec<WalletByBucketResponse>, i64) {
    let now = chrono::Utc::now();

    // BTreeMap keyed by (bucket_id, user_id) for deterministic output ordering.
    // `None` bucket_id sorts first; user_id breaks ties so the admin
    // (cross-user) view keeps one row per (user, bucket).
    let mut group_keys: BTreeMap<(Option<Uuid>, Uuid), ()> = BTreeMap::new();
    for wallet in &wallets {
        group_keys.insert((wallet.bucket_id, wallet.user_id), ());
    }

    let mut balances_by_key: BTreeMap<(Option<Uuid>, Uuid), BalancesByType> = BTreeMap::new();
    for (bucket_id, user_id) in group_keys.keys() {
        // Aggregate (bucket_id=None) rows have no concrete bucket; their
        // derived balance is the user-total across all buckets, fetched with
        // an empty bucket_ids slice. Concrete bucket rows restrict to that
        // single bucket_id.
        let bucket_filter: &[Uuid] = match bucket_id {
            Some(id) => std::slice::from_ref(id),
            None => &[],
        };
        let derived = state
            .points_repository
            .compute_available_balance(realm_id, *user_id, bucket_filter, now)
            .await
            .unwrap_or_default();
        balances_by_key.insert(
            (*bucket_id, *user_id),
            derived_to_balances_by_type(&derived),
        );
    }

    let mut cross_bucket_total: i64 = 0;
    let items = balances_by_key
        .into_iter()
        .map(|((bucket_id, user_id), balances_by_type)| {
            let bucket_total = balances_by_type.total();
            cross_bucket_total = cross_bucket_total.saturating_add(bucket_total);
            let (name, enabled) = match bucket_id.as_ref() {
                Some(id) => match bucket_dir.get(id) {
                    Some(d) => (Some(d.name.clone()), Some(d.enabled)),
                    None => (None, None),
                },
                None => (None, None),
            };
            WalletByBucketResponse {
                bucket_id,
                name,
                enabled,
                user_id,
                balances_by_type,
                bucket_total,
            }
        })
        .collect();

    (items, cross_bucket_total)
}

/// List all points wallets in a realm (admin only)
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/wallets",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("status" = Option<String>, Query, description = "Filter by wallet status"),
        ("search" = Option<String>, Query, description = "Search by user ID"),
        ("bucketId" = Option<String>, Query, description = "Filter by Credit Bucket ID"),
        ("page" = Option<u64>, Query, description = "Page number (0-based, default: 0)"),
        ("pageSize" = Option<u64>, Query, description = "Page size (default: 20, max: 100)")
    ),
    responses(
        (status = 200, description = "Wallets grouped by Credit Bucket", body = ListWalletsByBucketResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn list_wallets(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path(realm_id): Path<String>,
    Query(query): Query<ListWalletsQuery>,
) -> Result<ApiResult<ListWalletsByBucketResponse>, ApiError> {
    let user_id = require_authenticated_user_in_realm(&identity, &realm_id, "points wallets")?;
    require_permission(
        &state,
        &realm_id,
        &user_id.to_string(),
        "points",
        "view",
        "points.view",
    )
    .await?;

    // Optional bucket filter parsed up front so malformed input surfaces as 400.
    let bucket_filter = match query.bucket_id.as_deref().map(str::trim) {
        Some(s) if !s.is_empty() => Some(
            s.parse::<Uuid>()
                .map_err(|_| ApiError::bad_request("Invalid bucketId format"))?,
        ),
        _ => None,
    };

    let filters = WalletFilters {
        // user_id is left None here: the service hard-scopes non-managers to the
        // caller; managers keep None for the realm-wide (cross-user) view.
        user_id: None,
        bucket_id: bucket_filter,
        status: query.status,
        search: query.search,
        page: query.page,
        page_size: query.page_size,
    };

    match state
        .points_service
        .list_wallets(identity, &realm_id, filters)
        .await
    {
        Ok(paginated) => {
            // We discard the PageResponse shape here because the bucket-grouped
            // view is the contract. Pagination of the
            // underlying wallet rows is still honoured by the repository; the
            // grouping collapses the visible page.
            //
            // Enrich each bucket row with directory `name` / `enabled`. Loaded
            // once for the realm (not per row) to avoid N+1. Failures degrade
            // gracefully: rows keep `name`/`enabled` unset rather than 500ing
            // the balance view (the directory is a display-only enrichment).
            let bucket_dir = load_bucket_dir(&state, &realm_id).await;
            let (items, cross_bucket_total) =
                group_wallets_by_bucket(&state, &realm_id, paginated.data, &bucket_dir).await;
            Ok(ApiResult::ok(ListWalletsByBucketResponse {
                items,
                cross_bucket_total,
            }))
        }
        Err(e) => Err(ApiError::from(e)),
    }
}

/// Get points wallet for a specific user
#[utoipa::path(
    get,
    path = "/api/points/{realmId}/wallets/{userId}",
    params(
        ("realmId" = String, Path, description = "Realm ID"),
        ("userId" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Account retrieved successfully"),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
        (status = 404, description = "Not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Points"
)]
pub async fn get_wallet(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Path((realm_id, user_id)): Path<(String, String)>,
) -> Result<Json<PointsWalletResponse>, ApiError> {
    let _user_id = require_authenticated_user_in_realm(&identity, &realm_id, "points wallet")?;
    require_permission(
        &state,
        &realm_id,
        &_user_id.to_string(),
        "points",
        "view",
        "points.view",
    )
    .await?;

    let user_uuid = user_id
        .parse::<Uuid>()
        .map_err(|_| ApiError::bad_request("Invalid user ID format"))?;
    match state
        .points_service
        .get_wallet(identity, &realm_id, user_uuid)
        .await
    {
        Ok(account) => {
            // Derived SUM — same predicate as consumption, so
            // future-effective pre-grant rows are excluded and the visible
            // balance never leaks un-granted periods. Empty bucket_ids ⟺
            // aggregate across all the user's buckets (matches the user-total
            // view returned by get_wallet).
            let now = chrono::Utc::now();
            let derived = state
                .points_repository
                .compute_available_balance(&realm_id, user_uuid, &[], now)
                .await
                .map_err(ApiError::from)?;
            Ok(Json(wallet_to_response(account, derived)))
        }
        Err(e) => Err(ApiError::from(e)),
    }
}
