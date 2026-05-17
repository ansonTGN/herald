// =============================================================================
// PostgresBillingRepository Unit Tests
// =============================================================================
//
// Unit tests for PostgreSQL billing repository operations
//
// =============================================================================

use super::*;
use chrono::Utc;
use futures::FutureExt;
use herald_domain::billing::{
    BillingPeriod, BillingRepository, Product, Subscription, SubscriptionPlan, SubscriptionStatus,
    SubscriptionTier, test_helpers::*,
};
use herald_domain::common::entities::app_errors::CoreError;
use herald_test_db::{SharedTestDatabaseHandle, create_isolated_schema_database};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use sha2::{Digest, Sha256};
use std::future::Future;
use std::panic::{AssertUnwindSafe, resume_unwind};
use uuid::Uuid;

// =============================================================================
// Test Database Setup
// =============================================================================

struct BillingTestDb {
    db: DatabaseConnection,
    pool: sqlx::PgPool,
    schema: SharedTestDatabaseHandle,
}

impl BillingTestDb {
    async fn teardown(self) {
        let BillingTestDb { db, pool, schema } = self;
        drop(db);
        drop(pool);
        schema.teardown().await;
    }
}

async fn setup_test_db() -> BillingTestDb {
    let (schema, pool, db) = create_isolated_schema_database(3).await;

    create_test_realms(&db).await;
    create_test_products(&db).await;
    create_test_plans(&db).await;

    BillingTestDb { db, pool, schema }
}

async fn run_with_repo<F, Fut>(test_fn: F)
where
    F: FnOnce(PostgresBillingRepository) -> Fut,
    Fut: Future<Output = ()>,
{
    let test_db = setup_test_db().await;
    let repo = PostgresBillingRepository::new(test_db.db.clone());
    let result = AssertUnwindSafe(test_fn(repo)).catch_unwind().await;
    test_db.teardown().await;

    if let Err(panic_payload) = result {
        resume_unwind(panic_payload);
    }
}

/// Create test product records
async fn create_test_products(db: &DatabaseConnection) {
    let backend = db.get_database_backend();

    for realm_id in test_realm_ids() {
        let product_id = test_product_id_for_realm(realm_id);
        let _ = db
            .execute(Statement::from_string(
                backend,
                format!(
                    "INSERT INTO products (id, realm_id, code, title, description, enabled, created_at, updated_at) VALUES ('{}', '{}', '{}_product', 'Test Product', 'Test product for {}', true, NOW(), NOW()) ON CONFLICT (id) DO NOTHING",
                    product_id, realm_id, realm_id, realm_id
                ),
            ))
            .await;
    }
}

/// Create test plan records
async fn create_test_plans(db: &DatabaseConnection) {
    let backend = db.get_database_backend();

    for realm_id in test_realm_ids() {
        let plan_id = Uuid::now_v7();
        let product_id = test_product_id_for_realm(realm_id);
        let _ = db
            .execute(Statement::from_string(
                backend,
                format!(
                    "INSERT INTO subscription_plan (id, realm_id, name, title, description, type, price, currency, checkout_url, active, trial_days, sort_order, product_id, created_at, updated_at) VALUES ('{}', '{}', 'test_plan_{}', 'Test Plan', 'Test plan for {}', 'monthly', 2500, 'USD', NULL, true, 0, 1, '{}', NOW(), NOW()) ON CONFLICT DO NOTHING",
                    plan_id, realm_id, realm_id, realm_id, product_id
                ),
            ))
            .await;
    }
}

/// Create test realm records
async fn create_test_realms(db: &DatabaseConnection) {
    let backend = db.get_database_backend();

    let test_realm_ids = test_realm_ids();

    for realm_id in test_realm_ids {
        let _ = db
            .execute(Statement::from_string(
                backend,
                format!(
                    "INSERT INTO realm (id, name) VALUES ('{}', 'Test Realm {}') ON CONFLICT (id) DO NOTHING",
                    realm_id, realm_id
                ),
            ))
            .await;
    }
}

fn test_product_id_for_realm(realm_id: &str) -> Uuid {
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(
        &Sha256::digest(format!("herald-test-product:{realm_id}").as_bytes())[..16],
    );
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

/// Returns all test realm IDs used across tests
fn test_realm_ids() -> Vec<&'static str> {
    vec![
        "test_plan_move_persist",
        "test_public_visible",
        "test_public_hidden",
        "test_create_sub",
        "test_optional_fields",
        "test_find_by_realm",
        "test_find_by_creem",
        "test_update_status",
        "test_update_tier",
        "test_update_cancel",
        "test_different_realms_1",
        "test_different_realms_2",
        "test_idempotent",
        "test_realm_1",
        "test_realm_2",
        "test_realm_3",
        "test_create_event",
        "test_event_with_sub",
        "test_find_event",
        "test_mark_processed",
        "test_cancel_period",
        "test_idempotency",
        "test_find_realm",
        "test_find_creem",
        "test_nonexistent",
        "test_status_0",
        "test_status_1",
        "test_status_2",
        "test_status_3",
        "test_status_4",
        "test_status_5",
        "test_status_6",
        "test_tier_0",
        "test_tier_1",
        "test_tier_2",
        "test_tier_3",
    ]
}

// =============================================================================
// Subscription Tests
// =============================================================================

macro_rules! billing_repo_test {
    ($name:ident, |$repo:ident| $body:block) => {
        #[tokio::test]
        async fn $name() {
            run_with_repo(|$repo| async move $body).await;
        }
    };
}

billing_repo_test!(test_repository_create_subscription, |repo| {
    let subscription = test_subscription("test_create_sub");

    let result = repo.create_subscription(subscription.clone()).await;

    assert!(result.is_ok(), "Failed to create subscription");
    let created = result.unwrap();
    assert_eq!(created.realm_id, "test_create_sub");
    assert_subscription_status(&created, SubscriptionStatus::Active);
    assert_subscription_tier(&created, SubscriptionTier::Starter);
});

billing_repo_test!(
    test_repository_create_subscription_with_optional_fields,
    |repo| {
        let subscription = SubscriptionBuilder::new()
            .with_realm_id("test_optional_fields")
            .with_external_subscription_id("sub_test123")
            .with_external_product_id("prod_professional_yearly")
            .with_status(SubscriptionStatus::Trialing)
            .with_tier(SubscriptionTier::Professional)
            .with_period_end(Utc::now() + chrono::Duration::days(14))
            .with_cancel_at_period_end(true)
            .with_billing_period(BillingPeriod::Yearly)
            .build();

        let result = repo.create_subscription(subscription.clone()).await;

        assert!(result.is_ok(), "Failed to create subscription");
        let created = result.unwrap();
        assert_eq!(created.realm_id, "test_optional_fields");
        assert_subscription_status(&created, SubscriptionStatus::Trialing);
        assert_subscription_tier(&created, SubscriptionTier::Professional);
        assert_eq!(created.external_subscription_id, "sub_test123");
        assert!(created.cancel_at_period_end);
    }
);

billing_repo_test!(test_repository_find_by_realm_id_exists, |repo| {
    let subscription = test_subscription("test_find_realm");
    let expected_id = subscription.id;
    repo.create_subscription(subscription).await.unwrap();

    let result = repo.find_by_realm_id("test_find_realm").await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert!(found.is_some());
    let sub = found.unwrap();
    assert_eq!(sub.realm_id, "test_find_realm");
    assert_eq!(sub.id, expected_id);
    assert_subscription_status(&sub, SubscriptionStatus::Active);
});

billing_repo_test!(test_repository_find_by_realm_id_not_found, |repo| {
    let result = repo.find_by_realm_id("test_nonexistent_realm").await;

    assert!(result.is_ok());
    let found = result.unwrap();
    assert!(found.is_none());
});

billing_repo_test!(
    test_repository_find_by_external_subscription_id_exists,
    |repo| {
        let subscription = SubscriptionBuilder::new()
            .with_realm_id("test_find_creem")
            .with_external_subscription_id("creem_sub_test123")
            .build();
        repo.create_subscription(subscription).await.unwrap();

        let result = repo
            .find_by_external_subscription_id("creem_sub_test123", "creem")
            .await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let sub = found.unwrap();
        assert_eq!(sub.realm_id, "test_find_creem");
        assert_eq!(sub.external_subscription_id, "creem_sub_test123");
    }
);

billing_repo_test!(
    test_repository_find_by_external_subscription_id_not_found,
    |repo| {
        let result = repo
            .find_by_external_subscription_id("nonexistent_creem_sub", "creem")
            .await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_none());
    }
);

billing_repo_test!(test_repository_update_subscription_status, |repo| {
    let subscription = test_subscription("test_update_status");
    let mut created = repo.create_subscription(subscription).await.unwrap();

    created.status = SubscriptionStatus::Canceled;
    created.updated_at = Utc::now();

    let result = repo.update_subscription(created.clone()).await;

    assert!(result.is_ok());
    let updated = result.unwrap();
    assert_subscription_status(&updated, SubscriptionStatus::Canceled);
    assert_eq!(updated.realm_id, "test_update_status");
});

billing_repo_test!(test_repository_update_subscription_tier, |repo| {
    let subscription = test_subscription("test_update_tier");
    let mut created = repo.create_subscription(subscription).await.unwrap();

    created.tier = SubscriptionTier::Professional;
    created.updated_at = Utc::now();

    let result = repo.update_subscription(created.clone()).await;

    assert!(result.is_ok());
    let updated = result.unwrap();
    assert_subscription_tier(&updated, SubscriptionTier::Professional);
});

billing_repo_test!(
    test_repository_update_subscription_cancel_at_period_end,
    |repo| {
        let subscription = test_subscription("test_cancel_period");
        let mut created = repo.create_subscription(subscription).await.unwrap();

        created.cancel_at_period_end = true;
        created.updated_at = Utc::now();

        let result = repo.update_subscription(created.clone()).await;

        assert!(result.is_ok());
        let updated = result.unwrap();
        assert!(updated.cancel_at_period_end);
    }
);

billing_repo_test!(test_repository_update_nonexistent_subscription, |repo| {
    let subscription = test_subscription("test_nonexistent");
    let fake_id = Uuid::now_v7();

    let nonexistent_sub = Subscription {
        id: fake_id,
        ..subscription
    };

    let result = repo.update_subscription(nonexistent_sub).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CoreError::SubscriptionNotFound(id) => {
            assert_eq!(id, fake_id.to_string());
        }
        _ => panic!("Expected SubscriptionNotFound error"),
    }
});

billing_repo_test!(
    test_repository_multiple_subscriptions_different_realms,
    |repo| {
        let sub1 = repo
            .create_subscription(test_subscription("test_realm_1"))
            .await
            .unwrap();
        let sub2 = repo
            .create_subscription(test_subscription("test_realm_2"))
            .await
            .unwrap();
        let sub3 = repo
            .create_subscription(test_subscription("test_realm_3"))
            .await
            .unwrap();

        let found1 = repo
            .find_by_realm_id("test_realm_1")
            .await
            .unwrap()
            .unwrap();
        let found2 = repo
            .find_by_realm_id("test_realm_2")
            .await
            .unwrap()
            .unwrap();
        let found3 = repo
            .find_by_realm_id("test_realm_3")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found1.id, sub1.id);
        assert_eq!(found2.id, sub2.id);
        assert_eq!(found3.id, sub3.id);

        // Verify they are different
        assert_ne!(found1.id, found2.id);
        assert_ne!(found2.id, found3.id);
        assert_ne!(found1.id, found3.id);
    }
);

billing_repo_test!(test_repository_subscription_all_statuses, |repo| {
    let statuses = [
        SubscriptionStatus::Active,
        SubscriptionStatus::Canceled,
        SubscriptionStatus::Expired,
        SubscriptionStatus::Pending,
        SubscriptionStatus::Trialing,
        SubscriptionStatus::Paused,
    ];

    for (i, status) in statuses.iter().enumerate() {
        let realm_id = format!("test_status_{}", i);
        let sub = SubscriptionBuilder::new()
            .with_realm_id(realm_id)
            .with_status(status.clone())
            .build();

        let created = repo.create_subscription(sub).await.unwrap();
        assert_subscription_status(&created, status.clone());
    }
});

billing_repo_test!(test_repository_subscription_all_tiers, |repo| {
    let tiers = [
        SubscriptionTier::Free,
        SubscriptionTier::Starter,
        SubscriptionTier::Professional,
        SubscriptionTier::Enterprise,
    ];

    for (i, tier) in tiers.iter().enumerate() {
        let realm_id = format!("test_tier_{}", i);
        let sub = SubscriptionBuilder::new()
            .with_realm_id(realm_id)
            .with_tier(tier.clone())
            .build();

        let created = repo.create_subscription(sub).await.unwrap();
        assert_subscription_tier(&created, tier.clone());
    }
});

// =============================================================================
// Payment Event Tests
// =============================================================================

billing_repo_test!(test_repository_create_payment_event, |repo| {
    let event = test_payment_event("test_create_event", "evt_create123");

    let result = repo.create_payment_event(event.clone()).await;

    assert!(result.is_ok());
    let created = result.unwrap();
    assert_eq!(created.realm_id, "test_create_event");
    assert_eq!(created.external_event_id, "evt_create123");
    assert_eq!(created.payment_provider, "creem");
    assert_eq!(created.event_type, "subscription.paid");
    assert!(!created.processed);
});

billing_repo_test!(
    test_repository_create_payment_event_with_subscription,
    |repo| {
        let subscription = test_subscription("test_event_with_sub");
        let created_sub = repo.create_subscription(subscription).await.unwrap();

        let event = PaymentEventBuilder::new()
            .with_realm_id("test_event_with_sub")
            .with_external_event_id("evt_with_sub123")
            .with_payment_provider("creem")
            .with_subscription_id(created_sub.id)
            .build();

        let result = repo.create_payment_event(event).await;

        assert!(result.is_ok());
        let created = result.unwrap();
        assert_eq!(created.subscription_id, Some(created_sub.id));
    }
);

billing_repo_test!(
    test_repository_find_payment_event_by_creem_id_exists,
    |repo| {
        let event = test_payment_event("test_find_event", "evt_find123");
        repo.create_payment_event(event).await.unwrap();

        let result = repo
            .find_payment_event_by_external_id("evt_find123", "creem")
            .await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_some());
        let evt = found.unwrap();
        assert_eq!(evt.realm_id, "test_find_event");
        assert_eq!(evt.external_event_id, "evt_find123");
        assert_eq!(evt.payment_provider, "creem");
    }
);

billing_repo_test!(
    test_repository_find_payment_event_by_creem_id_not_found,
    |repo| {
        let result = repo
            .find_payment_event_by_external_id("nonexistent_event_id", "creem")
            .await;

        assert!(result.is_ok());
        let found = result.unwrap();
        assert!(found.is_none());
    }
);

billing_repo_test!(test_repository_mark_payment_event_processed, |repo| {
    let event = test_payment_event("test_mark_processed", "evt_mark123");
    let created = repo.create_payment_event(event).await.unwrap();

    assert!(!created.processed);

    let result = repo.mark_payment_event_processed(created.id).await;

    assert!(result.is_ok());

    // Verify the event is marked as processed
    let found = repo
        .find_payment_event_by_external_id("evt_mark123", "creem")
        .await
        .unwrap();
    assert!(found.is_some());
    assert!(found.unwrap().processed);
});

billing_repo_test!(test_repository_mark_processed_nonexistent_event, |repo| {
    let fake_id = Uuid::now_v7();

    let result = repo.mark_payment_event_processed(fake_id).await;

    assert!(result.is_err());
});

billing_repo_test!(test_repository_payment_event_idempotency, |repo| {
    let event1 = test_payment_event("test_idempotency", "evt_dup123");
    repo.create_payment_event(event1).await.unwrap();

    // Try to create another event with same creem_event_id
    let event2 = test_payment_event("test_idempotency", "evt_dup123");

    let result = repo.create_payment_event(event2).await;

    // Should fail due to UNIQUE constraint
    assert!(result.is_err());
});

billing_repo_test!(test_repository_update_plan_persists_product_id, |repo| {
    let realm_id = "test_plan_move_persist";
    let source_product_id = test_product_id_for_realm(realm_id);
    let target_product_id = Uuid::now_v7();

    repo.create_product(Product {
        id: target_product_id,
        realm_id: realm_id.to_string(),
        code: "target_product".to_string(),
        title: "Target Product".to_string(),
        description: None,
        enabled: true,
        plans_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
    .await
    .unwrap();

    let created = repo
        .create_subscription_plan(
            SubscriptionPlanBuilder::new()
                .with_realm_id(realm_id)
                .with_name("move_plan")
                .with_product_id(source_product_id)
                .build(),
        )
        .await
        .unwrap();

    let updated = repo
        .update_subscription_plan(SubscriptionPlan {
            product_id: target_product_id,
            updated_at: Utc::now(),
            ..created.clone()
        })
        .await
        .unwrap();

    assert_eq!(updated.product_id, target_product_id);

    let reloaded = repo
        .find_subscription_plan_by_id(created.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(reloaded.product_id, target_product_id);
});

billing_repo_test!(
    test_repository_public_plan_queries_hide_disabled_products,
    |repo| {
        let visible_realm = "test_public_visible";
        let hidden_realm = "test_public_hidden";

        // Use random UUIDs to avoid conflicts in parallel tests
        let visible_product_id = Uuid::now_v7();
        let hidden_product_id = Uuid::now_v7();

        // Create product records first
        repo.create_product(Product {
            id: visible_product_id,
            realm_id: visible_realm.to_string(),
            code: "visible_product".to_string(),
            title: "Visible Product".to_string(),
            description: None,
            enabled: true,
            plans_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

        repo.create_product(Product {
            id: hidden_product_id,
            realm_id: hidden_realm.to_string(),
            code: "hidden_product".to_string(),
            title: "Hidden Product".to_string(),
            description: None,
            enabled: true,
            plans_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
        .await
        .unwrap();

        let visible_plan = repo
            .create_subscription_plan(
                SubscriptionPlanBuilder::new()
                    .with_realm_id(visible_realm)
                    .with_name("visible_plan")
                    .with_product_id(visible_product_id)
                    .build(),
            )
            .await
            .unwrap();

        let hidden_plan = repo
            .create_subscription_plan(
                SubscriptionPlanBuilder::new()
                    .with_realm_id(hidden_realm)
                    .with_name("hidden_plan")
                    .with_product_id(hidden_product_id)
                    .build(),
            )
            .await
            .unwrap();

        repo.update_product(hidden_realm, hidden_product_id, None, None, Some(false))
            .await
            .unwrap();

        let visible_plans = repo
            .list_public_plans_by_realm(visible_realm)
            .await
            .unwrap();
        assert!(visible_plans.iter().any(|plan| plan.id == visible_plan.id));

        let hidden_plans = repo.list_public_plans_by_realm(hidden_realm).await.unwrap();
        assert!(hidden_plans.iter().all(|plan| plan.id != hidden_plan.id));

        let visible_lookup = repo
            .find_public_plan_by_id(visible_realm, visible_plan.id)
            .await
            .unwrap();
        assert!(visible_lookup.is_some());

        let hidden_lookup = repo
            .find_public_plan_by_id(hidden_realm, hidden_plan.id)
            .await
            .unwrap();
        assert!(hidden_lookup.is_none());
    }
);
