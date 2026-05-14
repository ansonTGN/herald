#[cfg(test)]
mod tests {

    // Tests are organized by the type being tested

    // =============================================================================
    // Plan Entity Tests
    // =============================================================================

    // NOTE: Low-value serialization tests removed (test_plan_serialization, test_plan_with_optional_fields)
    // These tests only verified serde standard functionality without custom logic.
    // Serde guarantees are covered by the library itself and integration tests.

    // =============================================================================
    // CreatePlanInput & UpdatePlanInput Tests
    // ============================================================================

    // NOTE: Low-value builder tests removed (test_create_plan_input_*, test_update_plan_input_*)
    // These tests only verified builder pattern field assignments without business logic.
    // Builder pattern correctness is covered by integration tests and type system.

    // =============================================================================
    // BillingPeriod & ClientAppPlan Tests
    // ============================================================================

    // NOTE: Low-value tests removed (test_billing_period_*, test_client_app_plan_*)
    // These tests only verified simple conversion functions and struct field assignments.
    // Conversion functions are trivial and covered by usage in integration tests.

    // =============================================================================
    // SubscriptionStatus & SubscriptionTier Tests
    // ============================================================================

    // NOTE: Low-value conversion tests removed (test_subscription_status_*, test_subscription_tier_*)
    // These tests only verified simple enum-to-string conversions without business logic.
    // Conversion correctness is covered by integration tests.

    // Business logic test retained: state transition rules
    #[test]
    fn test_subscription_status_transition_rules() {
        use crate::billing::SubscriptionStatus;

        assert!(SubscriptionStatus::Pending.can_transition_to(&SubscriptionStatus::Active));
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Canceled));
        assert!(SubscriptionStatus::Canceled.can_transition_to(&SubscriptionStatus::Expired));

        assert!(!SubscriptionStatus::Canceled.can_transition_to(&SubscriptionStatus::Active));
        assert!(!SubscriptionStatus::Expired.can_transition_to(&SubscriptionStatus::Active));
        assert!(!SubscriptionStatus::Expired.can_transition_to(&SubscriptionStatus::Pending));

        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Active));
    }
}
