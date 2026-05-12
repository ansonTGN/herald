// =============================================================================
// Payment and Subscription Assertions
// =============================================================================
//
// Specialized assertion functions for payment/subscription testing.
// Provides clear error messages and validation logic.
//
// =============================================================================

#![allow(dead_code)]

use crate::tests::schema_test_context::SchemaTestContext as TestContext;
use herald_core::domain::billing::entities::{BillingPeriod, Subscription, SubscriptionStatus};
use uuid::Uuid;

/// ============================================================================
/// Subscription Status Assertions
/// ============================================================================
/// Assert subscription has expected status
///
/// # Arguments
/// * `actual` - Actual subscription status
/// * `expected` - Expected subscription status
///
/// Panics with clear message if status doesn't match
pub fn assert_subscription_status(actual: SubscriptionStatus, expected: SubscriptionStatus) {
    assert_eq!(
        actual, expected,
        "Expected subscription status {:?}, but got {:?}",
        expected, actual
    );
}

/// Assert subscription has expected billing period
///
/// # Arguments
/// * `subscription` - Subscription to check
/// * `expected_period` - Expected billing period
pub fn assert_billing_period_correct(subscription: &Subscription, expected_period: BillingPeriod) {
    assert_eq!(
        subscription.billing_period, expected_period,
        "Expected billing period {:?}, but got {:?}",
        expected_period, subscription.billing_period
    );
}

/// Assert subscription has access based on status
///
/// # Arguments
/// * `status` - Subscription status
/// * `expected_access` - Whether subscription should have access
pub fn assert_subscription_has_access(status: SubscriptionStatus, expected_access: bool) {
    let has_access = status.has_access();
    assert_eq!(
        has_access, expected_access,
        "Expected subscription with status {:?} to have access={}, but has access={}",
        status, expected_access, has_access
    );
}

/// ============================================================================
/// Subscription History Assertions
/// ============================================================================
/// Assert subscription history contains specific event type
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID
/// * `event_type` - Event type to look for (e.g., "created", "canceled", "upgraded")
pub async fn assert_subscription_history_contains(
    ctx: &TestContext,
    subscription_id: Uuid,
    event_type: &str,
) {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscription_history
         WHERE subscription_id = $1 AND event_type = $2",
    )
    .bind(subscription_id)
    .bind(event_type)
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap();

    assert!(
        count > 0,
        "Expected subscription history to contain event type '{}', but found 0 occurrences",
        event_type
    );
}

/// Assert subscription history entry count
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID
/// * `expected_count` - Expected number of history entries
pub async fn assert_subscription_history_count(
    ctx: &TestContext,
    subscription_id: Uuid,
    expected_count: usize,
) {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM subscription_history WHERE subscription_id = $1")
            .bind(subscription_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap();

    assert_eq!(
        count as usize, expected_count,
        "Expected {} subscription history entries, but found {}",
        expected_count, count
    );
}

/// ============================================================================
/// Payment Event Assertions
/// ============================================================================
/// Assert payment event exists and is unique (idempotency check)
///
/// # Arguments
/// * `ctx` - Test context
/// * `creem_event_id` - Creem event ID
///
/// Panics if event doesn't exist or if multiple events found
pub async fn assert_payment_idempotent(ctx: &TestContext, creem_event_id: &str) {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
            .bind(creem_event_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap();

    assert_eq!(
        count, 1,
        "Expected exactly 1 payment event with creem_event_id '{}', but found {}",
        creem_event_id, count
    );
}

/// Assert payment event is processed
///
/// # Arguments
/// * `ctx` - Test context
/// * `creem_event_id` - Creem event ID
pub async fn assert_payment_event_processed(ctx: &TestContext, creem_event_id: &str) {
    let processed: bool =
        sqlx::query_scalar("SELECT processed FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
            .bind(creem_event_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap();

    assert!(
        processed,
        "Expected payment event '{}' to be processed, but it's not",
        creem_event_id
    );
}

/// Assert payment event has specific event type
///
/// # Arguments
/// * `ctx` - Test context
/// * `creem_event_id` - Creem event ID
/// * `expected_type` - Expected event type
pub async fn assert_payment_event_type(
    ctx: &TestContext,
    creem_event_id: &str,
    expected_type: &str,
) {
    let event_type: String =
        sqlx::query_scalar("SELECT event_type FROM payment_event WHERE external_event_id = $1 AND payment_provider = 'creem'")
            .bind(creem_event_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap();

    assert_eq!(
        event_type, expected_type,
        "Expected payment event type '{}', but got '{}'",
        expected_type, event_type
    );
}

/// ============================================================================
/// Subscription Period Assertions
/// ============================================================================
/// Assert subscription period is approximately correct (within tolerance)
///
/// # Arguments
/// * `subscription` - Subscription to check
/// * `expected_start` - Expected period start
/// * `expected_end` - Expected period end
/// * `tolerance_seconds` - Allowed tolerance in seconds
pub async fn assert_subscription_period_approx(
    ctx: &TestContext,
    subscription_id: Uuid,
    expected_start: chrono::DateTime<chrono::Utc>,
    expected_end: chrono::DateTime<chrono::Utc>,
    tolerance_seconds: i64,
) {
    let (current_period_start, current_period_end): (
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    ) = sqlx::query_as(
        "SELECT current_period_start, current_period_end FROM subscription WHERE id = $1",
    )
    .bind(subscription_id)
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap();

    let start_diff = (current_period_start - expected_start).num_seconds().abs();
    let end_diff = (current_period_end - expected_end).num_seconds().abs();

    assert!(
        start_diff <= tolerance_seconds,
        "Period start differs by {} seconds (tolerance: {} seconds)",
        start_diff,
        tolerance_seconds
    );

    assert!(
        end_diff <= tolerance_seconds,
        "Period end differs by {} seconds (tolerance: {} seconds)",
        end_diff,
        tolerance_seconds
    );
}

/// Assert subscription period duration is correct
///
/// # Arguments
/// * `start` - Period start
/// * `end` - Period end
/// * `expected_days` - Expected period duration in days
pub fn assert_subscription_period_duration(
    start: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
    expected_days: i64,
) {
    let duration_days = (*end - *start).num_days();

    assert_eq!(
        duration_days, expected_days,
        "Expected subscription period to be {} days, but got {} days",
        expected_days, duration_days
    );
}

/// ============================================================================
/// Subscription Field Assertions
/// ============================================================================
/// Assert subscription has expected plan
///
/// # Arguments
/// * `actual_plan_id` - Actual plan ID
/// * `expected_plan_id` - Expected plan ID
pub fn assert_subscription_plan(actual_plan_id: Uuid, expected_plan_id: Uuid) {
    assert_eq!(
        actual_plan_id, expected_plan_id,
        "Expected subscription to have plan_id {}, but got {}",
        expected_plan_id, actual_plan_id
    );
}

/// Assert subscription has expected client app
///
/// # Arguments
/// * `actual_client_app_id` - Actual client app ID
/// * `expected_client_app_id` - Expected client app ID
pub fn assert_subscription_client_app(actual_client_app_id: Uuid, expected_client_app_id: Uuid) {
    assert_eq!(
        actual_client_app_id, expected_client_app_id,
        "Expected subscription to have client_app_id {}, but got {}",
        expected_client_app_id, actual_client_app_id
    );
}

/// Assert subscription cancel_at_period_end flag
///
/// # Arguments
/// * `actual_value` - Actual flag value
/// * `expected_value` - Expected flag value
pub fn assert_subscription_cancel_at_period_end(actual_value: bool, expected_value: bool) {
    assert_eq!(
        actual_value, expected_value,
        "Expected cancel_at_period_end={}, but got {}",
        expected_value, actual_value
    );
}

/// ============================================================================
/// State Transition Assertions
/// ============================================================================
/// Assert state transition is valid
///
/// # Arguments
/// * `from_status` - Original status
/// * `to_status` - Target status
pub fn assert_valid_state_transition(
    from_status: SubscriptionStatus,
    to_status: SubscriptionStatus,
) {
    assert!(
        from_status.can_transition_to(&to_status),
        "Invalid state transition from {:?} to {:?}",
        from_status,
        to_status
    );
}

/// Assert state transition is invalid
///
/// # Arguments
/// * `from_status` - Original status
/// * `to_status` - Target status
pub fn assert_invalid_state_transition(
    from_status: SubscriptionStatus,
    to_status: SubscriptionStatus,
) {
    assert!(
        !from_status.can_transition_to(&to_status),
        "Expected state transition from {:?} to {:?} to be invalid, but it's valid",
        from_status,
        to_status
    );
}

/// ============================================================================
/// Refund Assertions
/// ============================================================================
/// Assert refund record exists for subscription
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID
/// * `refund_amount` - Expected refund amount in cents
pub async fn assert_refund_exists(ctx: &TestContext, subscription_id: Uuid, refund_amount: i32) {
    // Check payment_event table for refund event
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM payment_event
         WHERE subscription_id = $1
         AND event_type LIKE '%refund%'
         AND (payload->>'amount')::int = $2",
    )
    .bind(subscription_id)
    .bind(refund_amount)
    .fetch_one(&ctx.app_state.pool)
    .await
    .unwrap();

    assert!(
        count > 0,
        "Expected refund record of {} cents for subscription {}, but found none",
        refund_amount,
        subscription_id
    );
}

/// ============================================================================
/// Helper Functions
/// ============================================================================
/// Get subscription status from database by ID
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID
pub async fn get_subscription_status(
    ctx: &TestContext,
    subscription_id: Uuid,
) -> SubscriptionStatus {
    let status_str: String = sqlx::query_scalar("SELECT status FROM subscription WHERE id = $1")
        .bind(subscription_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();

    status_str.parse().unwrap()
}

/// Get subscription billing period from database by ID
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID
pub async fn get_subscription_billing_period(
    ctx: &TestContext,
    subscription_id: Uuid,
) -> BillingPeriod {
    let period_str: String =
        sqlx::query_scalar("SELECT billing_period FROM subscription WHERE id = $1")
            .bind(subscription_id)
            .fetch_one(&ctx.app_state.pool)
            .await
            .unwrap();

    BillingPeriod::from(period_str)
}

/// Count subscriptions by status
///
/// # Arguments
/// * `ctx` - Test context
/// * `status` - Subscription status to count
pub async fn count_subscriptions_by_status(ctx: &TestContext, status: &str) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM subscription WHERE status = $1")
        .bind(status)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap()
}

/// Check if subscription exists
///
/// # Arguments
/// * `ctx` - Test context
/// * `subscription_id` - Subscription ID
pub async fn subscription_exists(ctx: &TestContext, subscription_id: Uuid) -> bool {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM subscription WHERE id = $1")
        .bind(subscription_id)
        .fetch_one(&ctx.app_state.pool)
        .await
        .unwrap();
    count > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_subscription_status_passes() {
        assert_subscription_status(SubscriptionStatus::Active, SubscriptionStatus::Active);
    }

    #[test]
    #[should_panic(expected = "Expected subscription status")]
    fn test_assert_subscription_status_fails() {
        assert_subscription_status(SubscriptionStatus::Active, SubscriptionStatus::Canceled);
    }

    #[test]
    fn test_assert_valid_state_transition() {
        assert_valid_state_transition(SubscriptionStatus::Active, SubscriptionStatus::Canceled);
    }

    #[test]
    #[should_panic(expected = "Expected state transition")]
    fn test_assert_invalid_state_transition() {
        // Active -> Canceled is a valid transition, so this should panic
        // because we're asserting it should be invalid
        assert_invalid_state_transition(SubscriptionStatus::Active, SubscriptionStatus::Canceled);
    }

    #[test]
    fn test_assert_subscription_has_access() {
        assert_subscription_has_access(SubscriptionStatus::Active, true);
        assert_subscription_has_access(SubscriptionStatus::Canceled, false);
    }
}
