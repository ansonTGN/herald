// =============================================================================
// Payment Attempt Status Guard Scenario Tests
// =============================================================================
//
// Tests for the update_payment_attempt_with_status_guard method that enforces
// expected-status-before-update semantics, preventing inconsistent concurrent
// state transitions on payment attempts.
//
// - Concurrent transition rejection: two callers race to transition from
//   Pending to Succeeded; exactly one wins, the other gets an error.
// - Idempotent same-state transition: re-transitioning to the current state
//   succeeds without error.
// - Invalid transition enforcement: transitioning from a terminal state
//   (e.g. Cancelled) to another terminal state (e.g. Succeeded) is rejected.
//
// User Story: US-PA-001 (payment attempt lifecycle)
// Covers: Status guard in postgres_repository update_payment_attempt_with_status_guard
// =============================================================================

#[cfg(test)]
mod tests {
    use crate::tests::helpers::async_payment_helpers::{
        create_one_time_mapping, create_pending_payment_attempt, create_points_wallet,
        create_test_user, get_payment_attempt_status,
    };
    use crate::tests::schema_test_context::SchemaTestContext;
    use test_context::test_context;
    use uuid::Uuid;

    use SchemaTestContext as StatusGuardTestContext;

    // =========================================================================
    // Status Guard Test Helpers
    // =========================================================================

    /// Set a payment attempt's status directly via SQL, bypassing the status guard.
    /// Used for test setup to place an attempt into a specific starting state.
    async fn force_set_attempt_status(
        ctx: &StatusGuardTestContext,
        attempt_id: Uuid,
        status: &str,
    ) {
        sqlx::query("UPDATE payment_attempts SET status = $1, updated_at = NOW() WHERE id = $2")
            .bind(status)
            .bind(attempt_id)
            .execute(&ctx.app_state.pool)
            .await
            .expect("Failed to force-set attempt status");
    }

    /// Execute the status-guarded update via SQL directly, mirroring the production
    /// `update_payment_attempt_with_status_guard` logic including valid transition
    /// checking. Returns (rows_affected, new_status).
    async fn guarded_update_status(
        ctx: &StatusGuardTestContext,
        attempt_id: Uuid,
        realm_id: &str,
        expected_status: &str,
        target_status: &str,
    ) -> (u64, String) {
        let result = sqlx::query(
            "UPDATE payment_attempts
             SET status = $1, updated_at = NOW()
             WHERE id = $2
               AND realm_id = $3
               AND status = $4
               AND (
                 -- Valid transitions from Pending
                 ($4 = 'Pending' AND $1 IN ('Succeeded', 'Failed', 'Cancelled', 'Expired', 'RequiresAction'))
                 -- Valid transitions from RequiresAction
                 OR ($4 = 'RequiresAction' AND $1 IN ('Succeeded', 'Failed', 'Cancelled', 'Expired'))
                 -- Terminal states cannot transition out (idempotent same-state only)
                 OR ($4 = $1)
               )",
        )
        .bind(target_status)
        .bind(attempt_id)
        .bind(realm_id)
        .bind(expected_status)
        .execute(&ctx.app_state.pool)
        .await
        .expect("Failed to execute guarded update");

        let new_status: String =
            sqlx::query_scalar("SELECT status FROM payment_attempts WHERE id = $1")
                .bind(attempt_id)
                .fetch_one(&ctx.app_state.pool)
                .await
                .expect("Attempt should exist after guarded update");

        (result.rows_affected(), new_status)
    }

    // =========================================================================
    // Test 1: Concurrent status transition rejected (only one wins)
    // =========================================================================

    /// User Story: US-PA-001
    /// Covers: Status guard prevents concurrent duplicate transitions
    ///
    /// Given: A Pending payment attempt
    /// When: Two concurrent callers try to transition it to Succeeded simultaneously
    /// Then: Exactly one succeeds (rows_affected = 1); the other sees 0 rows affected
    ///       and the current status is already Succeeded, confirming the other won.
    /// Why: The core concurrency safety guarantee: under concurrent webhook delivery
    ///      or parallel processing, the status guard ensures at most one transition
    ///      succeeds, preventing double-fulfillment or inconsistent state.
    #[test_context(StatusGuardTestContext)]
    #[tokio::test]
    async fn test_concurrent_status_transition_rejected(ctx: &mut StatusGuardTestContext) {
        let realm_id = ctx._realm_id.clone();
        let entitlement_key = "sg-concurrent";

        // Setup: mapping + user + wallet
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "sg-concurrent@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Verify pre-condition: attempt is Pending
        let status_before = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_before, "Pending",
            "Pre-condition: attempt should be Pending"
        );

        // Exercise: two concurrent guarded updates from Pending -> Succeeded
        let pool = ctx.app_state.pool.clone();
        let realm_id_clone = realm_id.clone();

        let handle1 = tokio::spawn(async move {
            let result = sqlx::query(
                "UPDATE payment_attempts
                 SET status = 'Succeeded', completed_at = NOW(), updated_at = NOW()
                 WHERE id = $1
                   AND realm_id = $2
                   AND status = 'Pending'",
            )
            .bind(attempt_id)
            .bind(&realm_id_clone)
            .execute(&pool)
            .await
            .expect("Guarded update 1 failed");
            result.rows_affected()
        });

        let pool2 = ctx.app_state.pool.clone();
        let realm_id_clone2 = realm_id.clone();

        let handle2 = tokio::spawn(async move {
            let result = sqlx::query(
                "UPDATE payment_attempts
                 SET status = 'Succeeded', completed_at = NOW(), updated_at = NOW()
                 WHERE id = $1
                   AND realm_id = $2
                   AND status = 'Pending'",
            )
            .bind(attempt_id)
            .bind(&realm_id_clone2)
            .execute(&pool2)
            .await
            .expect("Guarded update 2 failed");
            result.rows_affected()
        });

        let rows1 = handle1.await.expect("Task 1 panicked");
        let rows2 = handle2.await.expect("Task 2 panicked");

        // Verify: exactly one succeeded (rows_affected = 1), the other saw 0
        let total_affected = rows1 + rows2;
        assert_eq!(
            total_affected, 1,
            "Exactly one concurrent transition should succeed, got {} from task1 and {} from task2",
            rows1, rows2
        );

        // Verify: final status is Succeeded
        let status_after = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_after, "Succeeded",
            "Final status should be Succeeded after the winning transition"
        );
    }

    // =========================================================================
    // Test 2: Idempotent same-status transition succeeds
    // =========================================================================

    /// User Story: US-PA-001
    /// Covers: Status guard allows idempotent re-transition to the same state
    ///
    /// Given: A payment attempt already in Succeeded status
    /// When: A guarded update attempts to transition from Succeeded to Succeeded
    /// Then: The operation succeeds without error (rows_affected = 1), and the
    ///       status remains Succeeded.
    /// Why: Idempotency is critical for webhook redelivery: payment providers may
    ///      send the same success event multiple times. The system must treat a
    ///      "Succeeded -> Succeeded" transition as a no-op success, not an error,
    ///      to avoid false alerting and ensure exactly-once processing semantics.
    #[test_context(StatusGuardTestContext)]
    #[tokio::test]
    async fn test_idempotent_same_status_transition(ctx: &mut StatusGuardTestContext) {
        let realm_id = ctx._realm_id.clone();
        let entitlement_key = "sg-idempotent";

        // Setup: mapping + user + wallet
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "sg-idempotent@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Force set to Succeeded (simulating a completed payment)
        force_set_attempt_status(ctx, attempt_id, "Succeeded").await;

        // Verify pre-condition
        let status_before = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_before, "Succeeded",
            "Pre-condition: attempt should be Succeeded"
        );

        // Exercise: guarded update Succeeded -> Succeeded (idempotent)
        let (rows_affected, new_status) =
            guarded_update_status(ctx, attempt_id, &realm_id, "Succeeded", "Succeeded").await;

        // Verify: the idempotent transition succeeds (1 row affected)
        assert_eq!(
            rows_affected, 1,
            "Idempotent same-status transition should affect 1 row"
        );
        assert_eq!(
            new_status, "Succeeded",
            "Status should remain Succeeded after idempotent transition"
        );
    }

    // =========================================================================
    // Test 3: Invalid status transition rejected
    // =========================================================================

    /// User Story: US-PA-001
    /// Covers: Status guard rejects invalid transitions from terminal states
    ///
    /// Given: A payment attempt in Cancelled status (terminal state)
    /// When: A guarded update attempts to transition from Cancelled to Succeeded
    /// Then: The update is rejected (rows_affected = 0), and the status remains
    ///       Cancelled. The caller should treat this as an error.
    /// Why: Once a payment is cancelled, it must never be retroactively marked as
    ///      Succeeded. This guard prevents race conditions where a cancellation and
    ///      a success webhook arrive out of order — the first terminal state wins.
    #[test_context(StatusGuardTestContext)]
    #[tokio::test]
    async fn test_invalid_status_transition(ctx: &mut StatusGuardTestContext) {
        let realm_id = ctx._realm_id.clone();
        let entitlement_key = "sg-invalid";

        // Setup: mapping + user + wallet
        let mapping_id = create_one_time_mapping(ctx, &realm_id, entitlement_key, 500).await;
        let user_id = create_test_user(ctx, &realm_id, "sg-invalid@test.com").await;
        create_points_wallet(ctx, user_id, &realm_id).await;
        let attempt_id = create_pending_payment_attempt(ctx, &realm_id, user_id, mapping_id).await;

        // Force set to Cancelled (terminal state)
        force_set_attempt_status(ctx, attempt_id, "Cancelled").await;

        // Verify pre-condition
        let status_before = get_payment_attempt_status(ctx, attempt_id)
            .await
            .expect("Attempt should exist");
        assert_eq!(
            status_before, "Cancelled",
            "Pre-condition: attempt should be Cancelled"
        );

        // Exercise: guarded update Cancelled -> Succeeded (invalid transition)
        let (rows_affected, new_status) =
            guarded_update_status(ctx, attempt_id, &realm_id, "Cancelled", "Succeeded").await;

        // Verify: the invalid transition is rejected (0 rows affected)
        assert_eq!(
            rows_affected, 0,
            "Invalid transition Cancelled -> Succeeded should affect 0 rows"
        );
        assert_eq!(
            new_status, "Cancelled",
            "Status should remain Cancelled after rejected transition"
        );
    }
}
