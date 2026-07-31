use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;
use crate::points::{
    DistributionEvent, DistributionRuleOwner, DistributionRuleSelection, DistributionTrigger,
    PointsRepository, event_key_for_registration,
};

pub struct RegistrationService<R>
where
    R: PointsRepository + Send + Sync,
{
    repository: Arc<R>,
}

impl<R> RegistrationService<R>
where
    R: PointsRepository + Send + Sync,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Execute the Realm's registration and free-periodic rules as one
    /// idempotent transaction. An empty rule set is still persisted as a
    /// completed event, so points configuration never changes registration
    /// success semantics.
    pub async fn handle_user_registration(
        &self,
        user_id: Uuid,
        realm_id: &str,
    ) -> Result<(), CoreError> {
        let (event, selection) = registration_execution(user_id, realm_id, Utc::now());
        let results = self
            .repository
            .execute_distribution_event_atomic(event, selection)
            .await?;

        tracing::info!(
            %realm_id,
            %user_id,
            result_count = results.len(),
            "registration points distribution completed"
        );
        Ok(())
    }
}

fn registration_execution(
    user_id: Uuid,
    realm_id: &str,
    registered_at: DateTime<Utc>,
) -> (DistributionEvent, DistributionRuleSelection) {
    let source_id = event_key_for_registration(user_id);
    (
        DistributionEvent {
            realm_id: realm_id.to_string(),
            user_id,
            owner: DistributionRuleOwner::RealmRegistration,
            trigger: DistributionTrigger::Registration,
            event_key: source_id.clone(),
            source_id,
            effective_from: registered_at,
            effective_until: None,
        },
        DistributionRuleSelection::CurrentOwnerRules,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Registration and free-periodic grants must share the registration event
    /// anchor so the repository can select and commit both rule sets together.
    #[test]
    fn registration_uses_one_current_owner_event() {
        let user_id = Uuid::from_u128(7);
        let at = DateTime::parse_from_rfc3339("2026-07-29T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let (event, selection) = registration_execution(user_id, "realm-a", at);

        assert_eq!(
            event.event_key,
            "registration:00000000-0000-0000-0000-000000000007"
        );
        assert_eq!(event.source_id, event.event_key);
        assert_eq!(event.owner, DistributionRuleOwner::RealmRegistration);
        assert_eq!(event.trigger, DistributionTrigger::Registration);
        assert_eq!(event.effective_from, at);
        assert_eq!(event.effective_until, None);
        assert!(matches!(
            selection,
            DistributionRuleSelection::CurrentOwnerRules
        ));
    }
}
