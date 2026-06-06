#[cfg(test)]
mod tests {
    #[test]
    fn test_subscription_status_transition_rules() {
        use crate::billing::SubscriptionStatus;

        assert!(SubscriptionStatus::Pending.can_transition_to(&SubscriptionStatus::Active));
        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Canceled));
        assert!(SubscriptionStatus::ScheduledCancel.can_transition_to(&SubscriptionStatus::Active));
        assert!(SubscriptionStatus::Canceled.can_transition_to(&SubscriptionStatus::Expired));

        assert!(!SubscriptionStatus::Canceled.can_transition_to(&SubscriptionStatus::Active));
        assert!(!SubscriptionStatus::Expired.can_transition_to(&SubscriptionStatus::Active));
        assert!(!SubscriptionStatus::Expired.can_transition_to(&SubscriptionStatus::Pending));

        assert!(SubscriptionStatus::Active.can_transition_to(&SubscriptionStatus::Active));
    }
}
