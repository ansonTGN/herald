use super::*;
use chrono::{Duration, Utc};
use futures::FutureExt;
use herald_domain::audit::{
    ActorType, AuditAction, AuditCategory, AuditEventFilters, AuditEventRepository, AuditResult,
    AuditTargetType, NewAuditEvent,
};
use herald_test_db::{SharedTestDatabaseHandle, create_isolated_schema_database};
use std::future::Future;
use std::panic::{AssertUnwindSafe, resume_unwind};
use uuid::Uuid;

struct AuditTestDb {
    db: sea_orm::DatabaseConnection,
    pool: sqlx::PgPool,
    schema: SharedTestDatabaseHandle,
}

impl AuditTestDb {
    async fn teardown(self) {
        let AuditTestDb { db, pool, schema } = self;
        drop(db);
        drop(pool);
        schema.teardown().await;
    }
}

async fn setup_test_db() -> AuditTestDb {
    let (schema, pool, db) = create_isolated_schema_database(3).await;
    AuditTestDb { db, pool, schema }
}

async fn run_with_repo<F, Fut>(test_fn: F)
where
    F: FnOnce(PostgresAuditEventRepository) -> Fut,
    Fut: Future<Output = ()>,
{
    let test_db = setup_test_db().await;
    let repo = PostgresAuditEventRepository::new(test_db.db.clone());
    let result = AssertUnwindSafe(test_fn(repo)).catch_unwind().await;
    test_db.teardown().await;

    if let Err(panic_payload) = result {
        resume_unwind(panic_payload);
    }
}

fn make_event(realm_id: &str) -> NewAuditEvent {
    NewAuditEvent {
        realm_id: realm_id.to_string(),
        category: AuditCategory::Auth,
        action: AuditAction::AuthLogin,
        actor_id: format!("actor_{}", Uuid::now_v7().to_string().get(..8).unwrap()),
        actor_type: Some(ActorType::User),
        actor_name: Some("test_actor".to_string()),
        target_type: AuditTargetType::Session,
        target_id: format!("target_{}", Uuid::now_v7().to_string().get(..8).unwrap()),
        target_name: Some("test_target".to_string()),
        result: AuditResult::Success,
        details: Some(serde_json::json!({ "method": "password" })),
        ip_address: Some("127.0.0.1".to_string()),
        user_agent: Some("test-agent/1.0".to_string()),
        trace_id: None,
    }
}

fn make_event_with_overrides(
    realm_id: &str,
    category: AuditCategory,
    action: AuditAction,
    actor_id: &str,
    result: AuditResult,
) -> NewAuditEvent {
    NewAuditEvent {
        realm_id: realm_id.to_string(),
        category,
        action,
        actor_id: actor_id.to_string(),
        actor_type: Some(ActorType::Admin),
        actor_name: Some(format!("actor_{}", actor_id)),
        target_type: AuditTargetType::User,
        target_id: format!("target_{}", Uuid::now_v7().to_string().get(..8).unwrap()),
        target_name: Some("test_target".to_string()),
        result,
        details: None,
        ip_address: None,
        user_agent: None,
        trace_id: Some(format!(
            "trace_{}",
            Uuid::now_v7().to_string().get(..8).unwrap()
        )),
    }
}

macro_rules! audit_repo_test {
    ($name:ident, |$repo:ident| $body:block) => {
        #[tokio::test]
        async fn $name() {
            run_with_repo(|$repo| async move $body).await;
        }
    };
}

audit_repo_test!(test_create_find_by_id_round_trip, |repo| {
    let realm_id = "audit_round_trip_realm";
    let new_event = NewAuditEvent {
        realm_id: realm_id.to_string(),
        category: AuditCategory::UserManagement,
        action: AuditAction::UserCreate,
        actor_id: "actor_abc123".to_string(),
        actor_type: Some(ActorType::Admin),
        actor_name: Some("admin_alice".to_string()),
        target_type: AuditTargetType::User,
        target_id: "target_user_456".to_string(),
        target_name: Some("bob".to_string()),
        result: AuditResult::Success,
        details: Some(serde_json::json!({ "email": "bob@example.com", "role": "user" })),
        ip_address: Some("192.168.1.100".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        trace_id: Some("trace_789xyz".to_string()),
    };

    let created = repo
        .create(new_event.clone())
        .await
        .expect("create should succeed");

    // Verify generated fields
    assert!(!created.id.is_nil(), "id should be generated");
    assert!(
        created.created_at <= Utc::now(),
        "created_at should be near now"
    );
    assert!(
        created.created_at >= Utc::now() - Duration::seconds(5),
        "created_at should be recent"
    );

    // Verify all stored fields match input
    assert_eq!(created.realm_id, new_event.realm_id);
    assert_eq!(created.category, new_event.category);
    assert_eq!(created.action, new_event.action);
    assert_eq!(created.actor_id, new_event.actor_id);
    assert_eq!(created.actor_type, new_event.actor_type);
    assert_eq!(created.actor_name, new_event.actor_name);
    assert_eq!(created.target_type, new_event.target_type);
    assert_eq!(created.target_id, new_event.target_id);
    assert_eq!(created.target_name, new_event.target_name);
    assert_eq!(created.result, new_event.result);
    assert_eq!(created.details, new_event.details);
    assert_eq!(created.ip_address, new_event.ip_address);
    assert_eq!(created.user_agent, new_event.user_agent);
    assert_eq!(created.trace_id, new_event.trace_id);

    // Retrieve by id and verify round-trip
    let found = repo
        .find_by_id(realm_id, created.id)
        .await
        .expect("find_by_id should succeed")
        .expect("event should be found");

    assert_eq!(found.id, created.id);
    assert_eq!(found.realm_id, created.realm_id);
    assert_eq!(found.category, created.category);
    assert_eq!(found.action, created.action);
    assert_eq!(found.actor_id, created.actor_id);
    assert_eq!(found.actor_type, created.actor_type);
    assert_eq!(found.actor_name, created.actor_name);
    assert_eq!(found.target_type, created.target_type);
    assert_eq!(found.target_id, created.target_id);
    assert_eq!(found.target_name, created.target_name);
    assert_eq!(found.result, created.result);
    assert_eq!(found.details, created.details);
    assert_eq!(found.ip_address, created.ip_address);
    assert_eq!(found.user_agent, created.user_agent);
    assert_eq!(found.trace_id, created.trace_id);
    // Timestamps match within a small tolerance
    let time_diff = (found.created_at - created.created_at)
        .num_milliseconds()
        .abs();
    assert!(
        time_diff < 100,
        "created_at should match within 100ms, got diff={}",
        time_diff
    );
});

audit_repo_test!(test_list_paginated_basic_ordering, |repo| {
    let realm_id = "audit_pagination_realm";

    // Insert 5 events with a small delay between them to ensure distinct created_at
    let mut created_ids = Vec::new();
    for i in 0..5 {
        let event = make_event(realm_id);
        let created = repo.create(event).await.expect("create should succeed");
        created_ids.push(created.id);
        // Small sleep to ensure distinct timestamps (UUIDv7 already gives ms precision)
        if i < 4 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    // Page 0, page_size 3
    let filters = AuditEventFilters {
        page: 0,
        page_size: 3,
        ..Default::default()
    };

    let result = repo
        .list_paginated(realm_id, filters)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result.total, 5, "total should be 5");
    assert_eq!(result.page, 0, "page should be 0");
    assert_eq!(result.page_size, 3, "page_size should be 3");
    assert_eq!(result.items.len(), 3, "should return 3 items on first page");

    // Verify DESC ordering by created_at (newest first)
    for window in result.items.windows(2) {
        assert!(
            window[0].created_at >= window[1].created_at,
            "items should be ordered by created_at DESC"
        );
    }

    // Page 1 should have the remaining 2
    let filters_page2 = AuditEventFilters {
        page: 1,
        page_size: 3,
        ..Default::default()
    };

    let result2 = repo
        .list_paginated(realm_id, filters_page2)
        .await
        .expect("list_paginated page 2 should succeed");

    assert_eq!(result2.total, 5);
    assert_eq!(result2.items.len(), 2, "second page should have 2 items");
});

audit_repo_test!(test_list_paginated_category_filter, |repo| {
    let realm_id = "audit_category_filter_realm";

    // Insert events with different categories
    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::UserManagement,
        AuditAction::UserCreate,
        "actor_1",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Auth,
        AuditAction::AuthLogin,
        "actor_2",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::RealmManagement,
        AuditAction::RealmCreate,
        "actor_3",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Auth,
        AuditAction::AuthLogout,
        "actor_4",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    // Filter by Auth category
    let filters = AuditEventFilters {
        category: Some(AuditCategory::Auth),
        ..Default::default()
    };

    let result = repo
        .list_paginated(realm_id, filters)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result.total, 2, "should find 2 Auth events");
    assert_eq!(result.items.len(), 2);
    assert!(
        result
            .items
            .iter()
            .all(|e| e.category == AuditCategory::Auth)
    );

    // Filter by UserManagement
    let filters_um = AuditEventFilters {
        category: Some(AuditCategory::UserManagement),
        ..Default::default()
    };

    let result_um = repo
        .list_paginated(realm_id, filters_um)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result_um.total, 1, "should find 1 UserManagement event");
    assert_eq!(result_um.items[0].category, AuditCategory::UserManagement);
});

audit_repo_test!(test_list_paginated_action_filter, |repo| {
    let realm_id = "audit_action_filter_realm";

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Rbac,
        AuditAction::RoleCreate,
        "actor_1",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Rbac,
        AuditAction::RoleDelete,
        "actor_2",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Rbac,
        AuditAction::PermissionCreate,
        "actor_3",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    let filters = AuditEventFilters {
        action: Some(AuditAction::RoleCreate),
        ..Default::default()
    };

    let result = repo
        .list_paginated(realm_id, filters)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result.total, 1, "should find exactly 1 RoleCreate event");
    assert_eq!(result.items[0].action, AuditAction::RoleCreate);
});

audit_repo_test!(test_list_paginated_actor_id_filter, |repo| {
    let realm_id = "audit_actor_filter_realm";

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Auth,
        AuditAction::AuthLogin,
        "actor_alice",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Auth,
        AuditAction::AuthLogin,
        "actor_bob",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Auth,
        AuditAction::AuthLoginFailed,
        "actor_alice",
        AuditResult::Failure,
    ))
    .await
    .unwrap();

    let filters = AuditEventFilters {
        actor_id: Some("actor_alice".to_string()),
        ..Default::default()
    };

    let result = repo
        .list_paginated(realm_id, filters)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result.total, 2, "should find 2 events by actor_alice");
    assert!(result.items.iter().all(|e| e.actor_id == "actor_alice"));
});

audit_repo_test!(test_list_paginated_time_range_filter, |repo| {
    let realm_id = "audit_time_filter_realm";

    // Insert an event now
    let event = make_event(realm_id);
    let created = repo.create(event).await.unwrap();

    // Create time boundaries around the event
    let before_event = created.created_at - Duration::minutes(5);
    let after_event = created.created_at + Duration::minutes(5);

    // Filter: range that includes the event
    let filters_inclusive = AuditEventFilters {
        start_time: Some(before_event),
        end_time: Some(after_event),
        ..Default::default()
    };

    let result = repo
        .list_paginated(realm_id, filters_inclusive)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result.total, 1, "should find 1 event in inclusive range");
    assert_eq!(result.items[0].id, created.id);

    // Filter: range that excludes the event (too early)
    let filters_early = AuditEventFilters {
        start_time: Some(before_event),
        end_time: Some(before_event + Duration::minutes(1)),
        ..Default::default()
    };

    let result_early = repo
        .list_paginated(realm_id, filters_early)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result_early.total, 0, "should find 0 events in early range");

    // Filter: only start_time
    let filters_start_only = AuditEventFilters {
        start_time: Some(before_event),
        ..Default::default()
    };

    let result_start = repo
        .list_paginated(realm_id, filters_start_only)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(
        result_start.total, 1,
        "should find 1 event with start_time only"
    );

    // Filter: only end_time
    let filters_end_only = AuditEventFilters {
        end_time: Some(after_event),
        ..Default::default()
    };

    let result_end = repo
        .list_paginated(realm_id, filters_end_only)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(
        result_end.total, 1,
        "should find 1 event with end_time only"
    );
});

audit_repo_test!(test_find_by_id_realm_isolation, |repo| {
    let realm_a = "audit_realm_isolation_a";
    let realm_b = "audit_realm_isolation_b";

    // Insert event in realm A
    let event_a = make_event(realm_a);
    let created_a = repo.create(event_a).await.unwrap();

    // Insert event in realm B
    let event_b = make_event(realm_b);
    let _created_b = repo.create(event_b).await.unwrap();

    // find_by_id with realm A's id should return the event
    let found = repo
        .find_by_id(realm_a, created_a.id)
        .await
        .expect("find_by_id should succeed");
    assert!(found.is_some(), "should find event in correct realm");
    assert_eq!(found.unwrap().id, created_a.id);

    // find_by_id with realm B's id should NOT return realm A's event
    let not_found = repo
        .find_by_id(realm_b, created_a.id)
        .await
        .expect("find_by_id should succeed");
    assert!(
        not_found.is_none(),
        "should NOT find realm A event when querying with realm B id"
    );

    // list_paginated should also be isolated
    let filters_a = AuditEventFilters {
        ..Default::default()
    };
    let result_a = repo.list_paginated(realm_a, filters_a).await.unwrap();
    assert_eq!(result_a.total, 1, "realm A should have exactly 1 event");
    assert_eq!(result_a.items[0].id, created_a.id);

    let filters_b = AuditEventFilters {
        ..Default::default()
    };
    let result_b = repo.list_paginated(realm_b, filters_b).await.unwrap();
    assert_eq!(result_b.total, 1, "realm B should have exactly 1 event");
    assert_ne!(result_b.items[0].id, created_a.id);
});

audit_repo_test!(test_list_paginated_empty_results, |repo| {
    let realm_id = "audit_empty_results_realm";

    // Insert some events
    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Auth,
        AuditAction::AuthLogin,
        "actor_1",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    // Filter with non-matching category
    let filters = AuditEventFilters {
        category: Some(AuditCategory::Rbac),
        ..Default::default()
    };

    let result = repo
        .list_paginated(realm_id, filters)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result.total, 0, "total should be 0 for non-matching filter");
    assert!(result.items.is_empty(), "items should be empty");

    // Filter with non-matching action
    let filters_action = AuditEventFilters {
        action: Some(AuditAction::RoleCreate),
        ..Default::default()
    };

    let result_action = repo
        .list_paginated(realm_id, filters_action)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result_action.total, 0);

    // Filter with non-matching actor_id
    let filters_actor = AuditEventFilters {
        actor_id: Some("nonexistent_actor".to_string()),
        ..Default::default()
    };

    let result_actor = repo
        .list_paginated(realm_id, filters_actor)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result_actor.total, 0);

    // Filter with past time range (before any events)
    let past_filters = AuditEventFilters {
        start_time: Some(Utc::now() - Duration::hours(2)),
        end_time: Some(Utc::now() - Duration::hours(1)),
        ..Default::default()
    };

    let result_past = repo
        .list_paginated(realm_id, past_filters)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(
        result_past.total, 0,
        "past time range should return 0 results"
    );

    // Non-existent realm
    let result_realm = repo
        .list_paginated("completely_nonexistent_realm", AuditEventFilters::default())
        .await
        .expect("list_paginated should succeed");

    assert_eq!(result_realm.total, 0);
    assert!(result_realm.items.is_empty());
});

audit_repo_test!(test_find_by_id_nonexistent, |repo| {
    let realm_id = "audit_find_nonexistent_realm";
    let fake_id = Uuid::now_v7();

    let result = repo
        .find_by_id(realm_id, fake_id)
        .await
        .expect("find_by_id should succeed");

    assert!(
        result.is_none(),
        "should return None for non-existent event"
    );
});

audit_repo_test!(test_list_paginated_combined_filters, |repo| {
    let realm_id = "audit_combined_filter_realm";

    // Insert events with varying properties
    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Auth,
        AuditAction::AuthLogin,
        "actor_combined_1",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::Auth,
        AuditAction::AuthLoginFailed,
        "actor_combined_1",
        AuditResult::Failure,
    ))
    .await
    .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    repo.create(make_event_with_overrides(
        realm_id,
        AuditCategory::UserManagement,
        AuditAction::UserCreate,
        "actor_combined_2",
        AuditResult::Success,
    ))
    .await
    .unwrap();

    // Filter: Auth category + actor_combined_1
    let filters = AuditEventFilters {
        category: Some(AuditCategory::Auth),
        actor_id: Some("actor_combined_1".to_string()),
        ..Default::default()
    };

    let result = repo
        .list_paginated(realm_id, filters)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(
        result.total, 2,
        "should find 2 Auth events by actor_combined_1"
    );

    // Filter: Auth category + Success result
    let filters_success = AuditEventFilters {
        category: Some(AuditCategory::Auth),
        action: Some(AuditAction::AuthLogin),
        ..Default::default()
    };

    let result_success = repo
        .list_paginated(realm_id, filters_success)
        .await
        .expect("list_paginated should succeed");

    assert_eq!(
        result_success.total, 1,
        "should find 1 successful AuthLogin event"
    );
    assert_eq!(result_success.items[0].result, AuditResult::Success);
});
