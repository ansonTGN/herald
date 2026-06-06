// =============================================================================
// Points System Test Fixtures
// =============================================================================
//
// Helper functions for creating test data in the Points system.
//
// **Usage**:
// ```rust
// use crate::tests::scenarios::points::fixtures::*;
//
// let user_id = create_test_user(&pool, &realm_id, "test@example.com").await;
// let wallet_id = create_test_points_wallet(&pool, user_id, 1000).await;
// ```
//
// =============================================================================

use herald_core::domain::client_api_keys::services::ClientApiKeyService;
use sqlx::PgPool;
use uuid::Uuid;

/// Create a test user with authentication
///
/// Returns the user ID
pub async fn create_test_user_with_auth(
    pool: &PgPool,
    realm_id: &str,
    email: &str,
    password: &str,
) -> Uuid {
    let user_uuid = Uuid::now_v7();
    let password_hash =
        bcrypt::hash(password, bcrypt::DEFAULT_COST).expect("Failed to hash password");

    sqlx::query(
        "INSERT INTO account (id, realm_id, email, password, status)
         VALUES ($1, $2, $3, $4, 1)",
    )
    .bind(user_uuid)
    .bind(realm_id)
    .bind(email)
    .bind(&password_hash)
    .execute(pool)
    .await
    .expect("Failed to create test user");

    // Assign user role to grant points.view permission
    // Get the user role
    let user_role_id: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM roles WHERE name = 'user' AND realm_id = $1 LIMIT 1")
            .bind(realm_id)
            .fetch_optional(pool)
            .await
            .expect("Failed to find user role");

    if let Some(role_id) = user_role_id {
        // Get the admin-web-console client_id for user role assignment
        let client_id: Option<String> =
            sqlx::query_scalar("SELECT client_id FROM client_app WHERE client_id = 'admin-web-console' AND realm_id = $1 LIMIT 1")
                .bind(realm_id)
                .fetch_optional(pool)
                .await
                .expect("Failed to find admin-web-console client_app");

        if let Some(client_id) = client_id {
            sqlx::query(
                "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $2::text)
                 ON CONFLICT DO NOTHING",
            )
            .bind(Uuid::now_v7())
            .bind(user_uuid)
            .bind(role_id)
            .bind(realm_id)
            .bind(&client_id)
            .bind(herald_core::domain::authorization::principal_types::USER)
            .execute(pool)
            .await
            .expect("Failed to assign user role");
        }
    }

    user_uuid
}

/// Create a test user
///
/// Returns the user ID
pub async fn create_test_user(pool: &PgPool, realm_id: &str, email: &str) -> Uuid {
    create_test_user_with_auth(pool, realm_id, email, "password123").await
}

/// Create a test admin user with RealmAdmin permissions
///
/// Returns the user ID
pub async fn create_test_admin(pool: &PgPool, realm_id: &str, email: &str) -> Uuid {
    let user_id = create_test_user_with_auth(pool, realm_id, email, "admin123").await;

    // Get the client_id for admin-web-console
    let client_id: String =
        sqlx::query_scalar("SELECT client_id FROM client_app WHERE client_id = 'admin-web-console' AND realm_id = $1 LIMIT 1")
            .bind(realm_id)
            .fetch_one(pool)
            .await
            .expect("Failed to find admin-web-console client_app");

    // Add RealmAdmin role to the user
    let role_id: Uuid =
        sqlx::query_scalar("SELECT id FROM roles WHERE name = 'realm-admin' AND realm_id = $1")
            .bind(realm_id)
            .fetch_one(pool)
            .await
            .expect("Failed to find realm-admin role");

    sqlx::query("INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id) VALUES ($1, $2, $3, $4, $5, $6, $2::text)")
        .bind(Uuid::now_v7())
        .bind(user_id)
        .bind(role_id)
        .bind(realm_id)
        .bind(&client_id)
        .bind(herald_core::domain::authorization::principal_types::USER)
        .execute(pool)
        .await
        .expect("Failed to assign realm-admin role");

    user_id
}

/// Create a test points account
///
/// Returns the account ID
///
/// NOTE: This creates both the points_wallets entry and the corresponding
/// points_credit_ledger entries to support the new ledger-based points system.
pub async fn create_test_points_wallet(pool: &PgPool, user_id: Uuid, balance: i64) -> Uuid {
    // Get user's realm_id
    let realm_id: String = sqlx::query_scalar("SELECT realm_id FROM account WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .expect("Failed to get user realm_id");

    let wallet_id = Uuid::now_v7();

    // Create the points_wallets entry with subscription_balance
    // (total_balance is computed as topup_balance + subscription_balance)
    sqlx::query(
        "INSERT INTO points_wallets (id, user_id, realm_id, subscription_balance, total_subscription_granted, topup_balance, total_topup_granted, total_recharged, total_consumed, status)
         VALUES ($1, $2, $3, $4, $4, 0, 0, 0, 0, 'active')",
    )
    .bind(wallet_id)
    .bind(user_id)
    .bind(&realm_id)
    .bind(balance)
    .execute(pool)
    .await
    .expect("Failed to create points account");

    // Create corresponding credit ledger entry for subscription credits
    // This is required for the new ledger-based points system
    let ledger_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO points_credit_ledger (id, user_id, realm_id, credit_type, source_type, source_id, granted_amount, used_amount, revoked_amount, status)
         VALUES ($1, $2, $3, 'subscription_credit', 'system_grant', $4, $5, 0, 0, 'active')",
    )
    .bind(ledger_id)
    .bind(user_id)
    .bind(&realm_id)
    .bind(format!("test-grant-{}", wallet_id))
    .bind(balance)
    .execute(pool)
    .await
    .expect("Failed to create credit ledger entry");

    wallet_id
}

/// Create a test transaction
///
/// Returns the transaction ID
#[allow(clippy::too_many_arguments)]
pub async fn create_test_transaction(
    pool: &PgPool,
    wallet_id: Uuid,
    user_id: Uuid,
    transaction_type: &str,
    amount: i64,
    balance_after: i64,
    description: Option<&str>,
    client_app_id: Option<Uuid>,
) -> Uuid {
    let realm_id: String = sqlx::query_scalar("SELECT realm_id FROM points_wallets WHERE id = $1")
        .bind(wallet_id)
        .fetch_one(pool)
        .await
        .expect("Failed to get account realm_id");

    let transaction_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO points_transactions (id, wallet_id, user_id, realm_id, type, amount, balance_after, description, client_app_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(transaction_id)
    .bind(wallet_id)
    .bind(user_id)
    .bind(&realm_id)
    .bind(transaction_type)
    .bind(amount)
    .bind(balance_after)
    .bind(description)
    .bind(client_app_id)
    .execute(pool)
    .await
    .expect("Failed to create transaction");

    transaction_id
}

/// Create a test plan (provider entitlement mapping)
///
/// Returns the plan ID
pub async fn create_test_plan(pool: &PgPool, realm_id: &str, plan_name: &str, price: i64) -> Uuid {
    let plan_id = Uuid::now_v7();
    let external_product_id = format!("prod_test_{}", plan_name);

    sqlx::query(
        "INSERT INTO provider_entitlement_mappings
            (id, realm_id, payment_provider, external_product_id, entitlement_key,
             points_per_period, grant_on_subscribe, enabled, created_at, updated_at)
         VALUES ($1, $2, 'creem', $3, $4, $5, true, true, NOW(), NOW())",
    )
    .bind(plan_id)
    .bind(realm_id)
    .bind(&external_product_id)
    .bind(plan_id.to_string())
    .bind(price)
    .execute(pool)
    .await
    .expect("Failed to create entitlement mapping");

    plan_id
}

/// Create a test plan configuration
///
/// Updates the existing provider_entitlement_mappings entry with points config
///
/// Returns the plan ID
pub async fn create_test_plan_config(
    pool: &PgPool,
    realm_id: &str,
    plan_id: Uuid,
    points_per_period: i64,
    validity_days: i64,
) -> Uuid {
    // Update the entitlement mapping with points config
    sqlx::query(
        "UPDATE provider_entitlement_mappings
         SET points_per_period = $1, validity_days = $2, updated_at = NOW()
         WHERE id = $3 AND realm_id = $4",
    )
    .bind(points_per_period)
    .bind(validity_days)
    .bind(plan_id)
    .bind(realm_id)
    .execute(pool)
    .await
    .expect("Failed to update plan config");

    plan_id
}

/// Create a test subscription
///
/// Returns the subscription ID
pub async fn create_test_subscription(
    pool: &PgPool,
    _user_id: Uuid,
    plan_id: Uuid,
    realm_id: &str,
) -> Uuid {
    let subscription_id = Uuid::now_v7();
    let client_app_id = Uuid::now_v7();
    let client_id = format!("client-{}", Uuid::now_v7());

    sqlx::query(
        "INSERT INTO client_app (id, realm_id, client_id, name, enabled)
         VALUES ($1, $2, $3, $4, true)",
    )
    .bind(client_app_id)
    .bind(realm_id)
    .bind(&client_id)
    .bind("Test Subscription Client App")
    .execute(pool)
    .await
    .expect("Failed to create client app for subscription");

    sqlx::query(
        "INSERT INTO subscription
            (id, realm_id, external_subscription_id, external_product_id, payment_provider,
             status, entitlement_key, current_period_start, current_period_end,
             cancel_at_period_end, client_app_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, 'creem', 'active', $5, NOW(), NOW() + INTERVAL '30 days',
                 false, $6, NOW(), NOW())",
    )
    .bind(subscription_id)
    .bind(realm_id)
    .bind(format!("ext-sub-{}", subscription_id))
    .bind("test_product_id")
    .bind(plan_id.to_string())
    .bind(client_app_id)
    .execute(pool)
    .await
    .expect("Failed to create subscription");

    subscription_id
}

/// Create a test API Key for a client app
///
/// Returns the API Key string
pub async fn create_test_api_key(pool: &PgPool, realm_id: &str, client_app_id: Uuid) -> String {
    // 1. Generate API Key (UUID v7) using Argon2-based service
    let api_key_plaintext = ClientApiKeyService::generate_api_key();

    // 2. First create a client app if it doesn't exist
    sqlx::query(
        "INSERT INTO client_app (id, realm_id, client_id, name, enabled)
         VALUES ($1, $2, $3, $4, true)
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(client_app_id)
    .bind(realm_id)
    .bind(format!("client-{}", client_app_id))
    .bind("Test Client App")
    .execute(pool)
    .await
    .expect("Failed to create client app");

    // 3. Hash API Key using Argon2 (same as production)
    let api_key_hash = ClientApiKeyService::hash_api_key(&api_key_plaintext);

    // 4. Store API key with all required fields including client_app_id
    let api_key_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO client_api_keys (id, realm_id, api_key_hash, client_app_id, name, enabled, created_at, last_used_at, usage_count)
         VALUES ($1, $2, $3, $4, $5, true, NOW(), NULL, 0)",
    )
    .bind(api_key_id)
    .bind(realm_id)
    .bind(&api_key_hash)
    .bind(client_app_id)
    .bind("Test API Key")
    .execute(pool)
    .await
    .expect("Failed to create API key");

    // 5. Grant ext endpoint permissions to the API key
    let client_id = format!("client-{}", client_app_id);
    grant_api_key_ext_permissions(pool, realm_id, &client_id, &api_key_id.to_string()).await;

    api_key_plaintext
}

/// Grant permissions needed by ext endpoints to an API key principal.
/// TODO: Consolidate with grant_api_key_permissions in client_helpers into a single shared implementation.
async fn grant_api_key_ext_permissions(
    pool: &PgPool,
    realm_id: &str,
    client_id: &str,
    api_key_id: &str,
) {
    let role_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO roles (id, name, realm_id, client_id, is_builtin) VALUES ($1, $2, $3, $4, false)"
    )
    .bind(role_id)
    .bind(format!("ext-api-key-role-{}", &api_key_id[..8]))
    .bind(realm_id)
    .bind(client_id)
    .execute(pool)
    .await
    .expect("Failed to create role for API key");

    for (resource, action) in [
        ("billing", "view"),
        ("points", "view"),
        ("points", "manage"),
    ] {
        sqlx::query(
            "INSERT INTO role_policies (id, role_id, realm_id, resource, action) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(Uuid::now_v7())
        .bind(role_id)
        .bind(realm_id)
        .bind(resource)
        .bind(action)
        .execute(pool)
        .await
        .expect("Failed to grant permission");
    }

    sqlx::query(
        "INSERT INTO user_roles (id, user_id, role_id, realm_id, client_id, principal_type, principal_id) VALUES ($1, NULL, $2, $3, $4, 'api_key', $5)"
    )
    .bind(Uuid::now_v7())
    .bind(role_id)
    .bind(realm_id)
    .bind(client_id)
    .bind(api_key_id)
    .execute(pool)
    .await
    .expect("Failed to assign role to API key");
}

/// Helper function to create a test client app
///
/// Returns the client app ID
pub async fn create_test_client_app(pool: &PgPool, realm_id: &str) -> Uuid {
    let client_app_id = Uuid::now_v7();
    let client_id = format!("client-{}", Uuid::now_v7());

    sqlx::query(
        "INSERT INTO client_app (id, realm_id, client_id, name, enabled)
         VALUES ($1, $2, $3, $4, true)",
    )
    .bind(client_app_id)
    .bind(realm_id)
    .bind(&client_id)
    .bind("Test Client App")
    .execute(pool)
    .await
    .expect("Failed to create client app");

    client_app_id
}
