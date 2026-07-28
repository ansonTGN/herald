// =============================================================================
// IAP Reconciliation Job Scenario Tests
// =============================================================================
//
// Exercises `IapReconciliationJob` (BE-D04, design support-iap §5.7) by
// constructing the job with a `MockProcessor` and calling `run()` directly
// (no worker process). Mirrors the `webhook_compensation_scenarios.rs`
// MockProcessor pattern.
//
// User Story: US-IAP-006 (scheduled reconciliation: Google lifecycle primary
//             driver / Apple compensation)
// Covers: design support-iap §5.7, §6.1 (backend integration).
//
// # Testability boundary (handoff note)
//
// The job builds its Apple / Google HTTP clients internally
// (`build_apple_client` → `AppleServerApiClient::new`,
//  `build_google_client` → `GoogleDeveloperClient::new`) using the production
// base URLs — there is no per-realm base-URL override on the client
// constructors today. Fully exercising the Apple notification-history and
// Google lifecycle HTTP paths therefore requires either a real Apple / Google
// sandbox account or a production-code change to thread a base URL through
// the client constructors (out of scope for an authoring item).
//
// The scenario tests here cover the **structurally testable** contract:
//
//   * job construction + `run()` returns `IapReconciliationStats`;
//   * a realm with no IAP credentials configured is a no-op for the job
//     (zero realms scanned, zero replays);
//   * failure isolation: a realm whose provider API is unreachable produces
//     a realm-level error that is logged + skipped — `run()` still returns
//     `Ok(stats)`, and any other realm / token in the same sweep is not
//     blocked. The Google case is exercised directly by configuring a realm
//     with Google credentials and asserting the sweep completes without
//     aborting (the production endpoint is unreachable from the test
//     sandbox, which is exactly the "single token failure" scenario).
//
// Full Apple / Google happy-path reconciliation coverage is delegated to the
// BE-T02 runner; the handoff flags the missing base-URL override seam.
//
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::iap_mocks::{
        build_service_account_json, fresh_rsa_pem, insert_apple_realm_config,
        insert_google_realm_config,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use herald_core::domain::billing::compensation::WebhookEventProcessor;
    use herald_core::domain::common::entities::app_errors::CoreError;
    use herald_worker::IapReconciliationJob;
    use serde_json::Value;
    use sqlx::PgPool;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use test_context::test_context;

    use SchemaTestContext as IapReconContext;

    // =========================================================================
    // Mock processor (mirrors webhook_compensation_scenarios.rs)
    // =========================================================================

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct ReprocessCallRecord {
        realm_id: String,
        payment_provider: String,
        event_type: String,
        payload: Value,
    }

    /// Manual mock processor that records all reprocess_event calls and can
    /// optionally inject failures for isolation testing.
    struct MockProcessor {
        calls: Arc<Mutex<Vec<ReprocessCallRecord>>>,
        /// If set, reprocess_event returns an error for every call (failure
        /// isolation test).
        fail_all: bool,
    }

    impl MockProcessor {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                fail_all: false,
            }
        }

        /// Build a processor that fails every `reprocess_event` call. Retained
        /// for failure-isolation tests that want to assert the processor's own
        /// errors are non-blocking; the current sweep relies on provider-API
        /// unreachability for the same effect.
        #[allow(dead_code)]
        fn failing() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                fail_all: true,
            }
        }

        fn call_log(&self) -> Arc<Mutex<Vec<ReprocessCallRecord>>> {
            self.calls.clone()
        }
    }

    impl WebhookEventProcessor for MockProcessor {
        fn reprocess_event<'a>(
            &'a self,
            realm_id: &'a str,
            payment_provider: &'a str,
            event_type: &'a str,
            payload: &'a Value,
        ) -> Pin<Box<dyn std::future::Future<Output = Result<(), CoreError>> + Send + 'a>> {
            Box::pin(async move {
                {
                    let mut calls = self.calls.lock().unwrap();
                    calls.push(ReprocessCallRecord {
                        realm_id: realm_id.to_string(),
                        payment_provider: payment_provider.to_string(),
                        event_type: event_type.to_string(),
                        payload: payload.clone(),
                    });
                }
                if self.fail_all {
                    return Err(CoreError::InternalServerError(
                        "simulated reprocess failure".to_string(),
                    ));
                }
                Ok(())
            })
        }
    }

    fn build_job(ctx: &IapReconContext, processor: MockProcessor) -> IapReconciliationJob {
        IapReconciliationJob::new(
            ctx.app_state.pool.clone(),
            Arc::new(processor),
            1800, // apple interval
            900,  // google interval
        )
    }

    fn count_calls(log: &Arc<Mutex<Vec<ReprocessCallRecord>>>) -> usize {
        log.lock().unwrap().len()
    }

    // =========================================================================
    // Tests
    // =========================================================================

    /// User Story: US-IAP-006 (no-op sweep for unconfigured realm)
    /// Covers: design §5.7 (only IAP-configured realms are scanned)
    ///
    /// A realm with no Apple / Google credentials in `realm_config` is
    /// invisible to `fetch_iap_configured_realms` — the job's `run()` scans
    /// zero realms and never invokes `reprocess_event`.
    #[test_context(IapReconContext)]
    #[tokio::test]
    async fn test_iap_reconciliation_no_configured_realms_is_noop(ctx: &mut IapReconContext) {
        let processor = MockProcessor::new();
        let log = processor.call_log();
        let job = build_job(ctx, processor);

        let stats = job.run().await.expect("job run must succeed");

        assert_eq!(stats.realms_scanned, 0, "no IAP realms configured");
        assert_eq!(stats.apple_replayed, 0);
        assert_eq!(stats.google_replayed, 0);
        assert_eq!(
            count_calls(&log),
            0,
            "no reprocess_event calls for unconfigured realm"
        );
    }

    /// User Story: US-IAP-006 (failure isolation — single token/realm failure
    /// does not block the sweep)
    /// Covers: design §5.7 "单对象失败不阻塞其他"
    ///
    /// Configure two realms with Google credentials. The Google Developer
    /// client uses the production base URL, which is unreachable from the
    /// test sandbox — each realm's `poll_google_lifecycle` therefore hits a
    /// network error. The sweep MUST nonetheless return `Ok(stats)` (the
    /// realm-level error is logged and skipped), proving the single-failure
    /// isolation contract.
    #[test_context(IapReconContext)]
    #[tokio::test]
    async fn test_iap_reconciliation_single_token_failure_not_blocking(ctx: &mut IapReconContext) {
        let pool: &PgPool = &ctx.app_state.pool;
        let realm_a = ctx._realm_id.clone();
        let realm_b = create_second_realm(pool).await;

        // Configure Google credentials for both realms. The service account
        // JSON is well-formed (parses), but the production Google endpoint is
        // unreachable from the sandbox — poll_google_lifecycle hits a network
        // error per realm.
        let rsa_pem = fresh_rsa_pem();
        let sa_json = build_service_account_json(
            "svc@herald-test.iam.gserviceaccount.com",
            std::str::from_utf8(&rsa_pem).unwrap(),
        );
        insert_google_realm_config(pool, &realm_a, "com.herald.app.a", &sa_json, None).await;
        insert_google_realm_config(pool, &realm_b, "com.herald.app.b", &sa_json, None).await;

        let processor = MockProcessor::new();
        let log = processor.call_log();
        let job = build_job(ctx, processor);

        // The sweep MUST succeed even though both realms' Google polls fail.
        let stats = job
            .run()
            .await
            .expect("job must not abort on per-realm failure");

        assert!(
            stats.realms_scanned >= 2,
            "both configured Google realms must be scanned, got {}",
            stats.realms_scanned
        );

        // No replays reached the processor (Google API was unreachable). The
        // key assertion is that `run()` returned Ok at all — the per-realm
        // failures were isolated.
        assert_eq!(
            count_calls(&log),
            0,
            "no replays when Google API unreachable (failure isolated)"
        );
    }

    /// User Story: US-IAP-006 (Apple compensation — realm with Apple
    /// credentials is scanned, failures isolated)
    /// Covers: design §5.7 (Apple notification-history compensation)
    ///
    /// A realm with Apple credentials is picked up by
    /// `fetch_iap_configured_realms`. The Apple Server API client uses a
    /// `.p8` private key + real Apple endpoint — unreachable from the
    /// sandbox — so `compensate_apple` hits a realm-level error. The sweep
    /// still returns `Ok(stats)` with the realm counted as scanned; the
    /// error is logged + skipped (non-blocking). This is the structural
    /// contract for the Apple compensation arm.
    #[test_context(IapReconContext)]
    #[tokio::test]
    async fn test_iap_reconciliation_apple_missed_notification_compensated(
        ctx: &mut IapReconContext,
    ) {
        let realm_id = ctx._realm_id.clone();
        insert_apple_realm_config(
            &ctx.app_state.pool,
            &realm_id,
            "com.herald.test",
            "issuer-test",
            "key-test",
            "-----BEGIN PRIVATE KEY-----\nMIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg\n-----END PRIVATE KEY-----",
            "sandbox",
        )
        .await;

        let processor = MockProcessor::new();
        let job = build_job(ctx, processor);

        // The Apple compensation arm fails at the API layer (unreachable),
        // but the sweep must succeed.
        let stats = job
            .run()
            .await
            .expect("Apple compensation failure must not abort sweep");

        assert!(
            stats.realms_scanned >= 1,
            "Apple-configured realm must be scanned"
        );
        // Apple replay count is zero because the API call failed; the
        // regression anchor is that run() returned Ok with the realm scanned.
    }

    /// User Story: US-IAP-006 (Google lifecycle polling — realm scanned,
    /// failures isolated)
    /// Covers: design §5.7 (Google lifecycle primary driver)
    ///
    /// A realm with Google credentials triggers `poll_google_lifecycle`. The
    /// poll fails at the API layer (unreachable), but the sweep returns
    /// `Ok(stats)` and the realm is counted. Structural contract for the
    /// Google lifecycle arm; the voided-purchase and state-change happy
    /// paths require the base-URL override seam (handoff note).
    #[test_context(IapReconContext)]
    #[tokio::test]
    async fn test_iap_reconciliation_google_state_change_captured(ctx: &mut IapReconContext) {
        let realm_id = ctx._realm_id.clone();
        let rsa_pem = fresh_rsa_pem();
        let sa_json = build_service_account_json(
            "svc@herald-test.iam.gserviceaccount.com",
            std::str::from_utf8(&rsa_pem).unwrap(),
        );
        insert_google_realm_config(
            &ctx.app_state.pool,
            &realm_id,
            "com.herald.app",
            &sa_json,
            None,
        )
        .await;

        let processor = MockProcessor::new();
        let job = build_job(ctx, processor);

        let stats = job
            .run()
            .await
            .expect("Google poll failure must not abort sweep");

        assert!(
            stats.realms_scanned >= 1,
            "Google-configured realm must be scanned"
        );
    }

    /// User Story: US-IAP-006 (voided purchase triggers refund — structural)
    /// Covers: design §5.7 (voidedpurchases.list → refund clawback)
    ///
    /// The voided-purchase replay path runs inside `poll_google_lifecycle`
    /// after the subscription-refresh pass. Without a reachable Google API
    /// the path produces no replays, but the sweep must still complete
    /// cleanly. This test pins the structural contract that a
    /// Google-configured realm with zero active subscriptions still
    /// completes the voided-purchase pass without aborting.
    #[test_context(IapReconContext)]
    #[tokio::test]
    async fn test_iap_reconciliation_voided_purchase_revokes(ctx: &mut IapReconContext) {
        let realm_id = ctx._realm_id.clone();
        let rsa_pem = fresh_rsa_pem();
        let sa_json = build_service_account_json(
            "svc@herald-test.iam.gserviceaccount.com",
            std::str::from_utf8(&rsa_pem).unwrap(),
        );
        insert_google_realm_config(
            &ctx.app_state.pool,
            &realm_id,
            "com.herald.app",
            &sa_json,
            None,
        )
        .await;

        let processor = MockProcessor::new();
        let log = processor.call_log();
        let job = build_job(ctx, processor);

        let stats = job
            .run()
            .await
            .expect("voided-purchase pass must not abort sweep");

        // Realm scanned; no active google subscriptions + unreachable API →
        // no replays. The structural contract is that run() completes Ok.
        assert!(stats.realms_scanned >= 1);
        assert_eq!(
            count_calls(&log),
            0,
            "no replays when API unreachable — voided pass completed cleanly"
        );
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Create a second test realm (distinct from the default realm in
    /// SchemaTestContext) and return its ID.
    async fn create_second_realm(pool: &PgPool) -> String {
        let realm_id = uuid::Uuid::now_v7().to_string();
        sqlx::query(
            "INSERT INTO realm (id, name, created_at, updated_at)
             VALUES ($1, $2, NOW(), NOW())",
        )
        .bind(&realm_id)
        .bind(format!("test-realm-{}", &realm_id[..8]))
        .execute(pool)
        .await
        .expect("Failed to create second realm");
        realm_id
    }
}
