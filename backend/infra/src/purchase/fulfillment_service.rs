// Purchase fulfillment service implementation

use std::sync::Arc;

use herald_domain::billing::{BillingRepository, Subscription, SubscriptionStatus};
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::payment_attempt::PaymentAttempt;
use herald_domain::points::{CreditSourceType, CreditType, PointsRepository};
use herald_domain::purchase::{
    FulfillmentResult, FulfillmentService, FulfillmentType, PointsGrant,
};

fn billing_period_to_days(period: Option<&str>) -> i64 {
    match period.map(|p| p.trim().to_ascii_lowercase()).as_deref() {
        Some("daily") | Some("day") => 1,
        Some("weekly") | Some("week") => 7,
        Some("monthly") | Some("month") => 30,
        Some("quarterly") | Some("quarter") => 90,
        Some("yearly") | Some("annual") | Some("annually") | Some("year") => 365,
        _ => 30,
    }
}

/// Implementation of fulfillment service for unified purchase handling
pub struct PostgresFulfillmentService<P, B>
where
    P: PointsRepository,
    B: BillingRepository,
{
    points_repository: Arc<P>,
    billing_repository: Arc<B>,
}

impl<P, B> PostgresFulfillmentService<P, B>
where
    P: PointsRepository,
    B: BillingRepository,
{
    pub fn new(points_repository: Arc<P>, billing_repository: Arc<B>) -> Self {
        Self {
            points_repository,
            billing_repository,
        }
    }
}

impl<P, B> FulfillmentService for PostgresFulfillmentService<P, B>
where
    P: PointsRepository + Send + Sync,
    B: BillingRepository + Send + Sync,
{
    async fn fulfill_subscription_purchase(
        &self,
        attempt: &PaymentAttempt,
        provider_transaction_id: String,
    ) -> Result<FulfillmentResult, CoreError> {
        tracing::info!(
            payment_attempt_id = %attempt.id,
            realm_id = %attempt.realm_id,
            user_id = %attempt.user_id,
            target_id = %attempt.target_id,
            "Fulfilling subscription purchase"
        );

        // Check for existing subscription by external subscription ID (idempotency check)
        if let Some(existing_subscription) = self
            .billing_repository
            .find_by_external_subscription_id(&provider_transaction_id, &attempt.payment_provider)
            .await?
        {
            tracing::info!(
                payment_attempt_id = %attempt.id,
                existing_subscription_id = %existing_subscription.id,
                "Existing subscription found for payment attempt, returning existing fulfillment"
            );

            return Ok(FulfillmentResult {
                fulfillment_type: FulfillmentType::SubscriptionCreated,
                subscription_id: Some(existing_subscription.id),
                points_granted: None,
                granted_at: existing_subscription.created_at,
            });
        }

        // Look up entitlement mapping by ID with realm isolation check
        let mapping = self
            .billing_repository
            .find_entitlement_mapping_by_id(attempt.target_id)
            .await?
            .filter(|m| m.realm_id == attempt.realm_id)
            .ok_or_else(|| {
                CoreError::not_found(&format!(
                    "Entitlement mapping {} for subscription fulfillment",
                    attempt.target_id
                ))
            })?;

        let entitlement_key = mapping.entitlement_key.clone();

        // A8: fulfillment routes by the `payment_attempt.bucket_id` snapshot taken
        // at purchase creation (§5.3). Live `mapping.bucket_id` is intentionally
        // NOT consulted here — mapping re-bucketing must not affect in-flight
        // attempts (A7).
        let bucket_id = attempt.bucket_id;

        let now = chrono::Utc::now();
        let period_days = billing_period_to_days(mapping.billing_period.as_deref());
        let period_end = now + chrono::Duration::days(period_days);

        // Create new subscription
        let subscription = Subscription {
            id: uuid::Uuid::now_v7(),
            realm_id: attempt.realm_id.clone(),
            user_id: Some(attempt.user_id),
            external_subscription_id: provider_transaction_id.clone(),
            external_product_id: attempt.target_id.to_string(),
            payment_provider: attempt.payment_provider.clone(),
            status: SubscriptionStatus::Active,
            entitlement_key: entitlement_key.clone(),
            external_price_id: mapping.external_price_id.clone(),
            bucket_id,
            provider_metadata: None,
            synced_at: Some(now),
            current_period_start: Some(now),
            current_period_end: Some(period_end),
            cancel_at_period_end: false,
            client_app_id: None,
            cancel_at: None,
            created_at: now,
            updated_at: now,
        };

        tracing::info!(
            subscription_id = %subscription.id,
            realm_id = %subscription.realm_id,
            user_id = ?subscription.user_id,
            entitlement_key = %entitlement_key,
            period_days,
            "Creating new subscription from payment attempt"
        );

        // Create subscription in database
        let created_subscription = self
            .billing_repository
            .create_subscription(subscription)
            .await?;

        tracing::info!(
            subscription_id = %created_subscription.id,
            "Subscription created successfully"
        );

        // Grant subscription credits if mapping is configured for it
        let points_granted = if mapping.grant_on_subscribe {
            match mapping.points_per_period {
                Some(points) if points > 0 => {
                    // bucket_id snapshot already resolved above (A8); pass through.
                    let credit_ledger = self
                        .points_repository
                        .grant_points_atomic(
                            &attempt.realm_id,
                            attempt.user_id,
                            bucket_id,
                            CreditType::SubscriptionCredit,
                            CreditSourceType::SubscriptionInitial,
                            points,
                            Some(period_end),
                            Some(entitlement_key.clone()),
                            None,
                            Some(format!("subscription_initial_grant:{}", attempt.id)),
                        )
                        .await?;

                    tracing::info!(
                        subscription_id = %created_subscription.id,
                        user_id = %attempt.user_id,
                        points,
                        "Subscription credits granted on subscribe"
                    );

                    Some(PointsGrant {
                        transaction_id: credit_ledger.id,
                        points_type: "subscription_credit".to_string(),
                        points,
                        description: format!(
                            "Subscription grant: {} points for {}",
                            points, entitlement_key
                        ),
                    })
                }
                _ => {
                    tracing::info!(
                        subscription_id = %created_subscription.id,
                        "No points_per_period configured, skipping credit grant"
                    );
                    None
                }
            }
        } else {
            tracing::info!(
                subscription_id = %created_subscription.id,
                "grant_on_subscribe is false, skipping credit grant"
            );
            None
        };

        Ok(FulfillmentResult {
            fulfillment_type: FulfillmentType::SubscriptionCreated,
            subscription_id: Some(created_subscription.id),
            points_granted,
            granted_at: created_subscription.created_at,
        })
    }

    async fn fulfill_one_time_purchase(
        &self,
        attempt: &PaymentAttempt,
        provider_transaction_id: String,
    ) -> Result<FulfillmentResult, CoreError> {
        tracing::info!(
            payment_attempt_id = %attempt.id,
            realm_id = %attempt.realm_id,
            user_id = %attempt.user_id,
            target_id = %attempt.target_id,
            "Fulfilling one-time purchase"
        );

        // Idempotency: check ledger by source_id first
        if let Some(existing_ledger) = self
            .points_repository
            .find_ledger_by_source_id(&attempt.realm_id, &attempt.id.to_string())
            .await?
        {
            tracing::info!(
                payment_attempt_id = %attempt.id,
                ledger_id = %existing_ledger.id,
                "Existing credit ledger found for payment attempt, returning existing fulfillment"
            );

            return Ok(FulfillmentResult {
                fulfillment_type: FulfillmentType::PointsGranted,
                subscription_id: None,
                points_granted: Some(PointsGrant {
                    transaction_id: existing_ledger.id,
                    points_type: "topup_credit".to_string(),
                    points: existing_ledger.granted_amount,
                    description: format!(
                        "One-time purchase (Payment: {})",
                        provider_transaction_id
                    ),
                }),
                granted_at: existing_ledger.created_at,
            });
        }

        // Read mapping from billing_repository by target_id with realm isolation check
        let mapping = self
            .billing_repository
            .find_entitlement_mapping_by_id(attempt.target_id)
            .await?
            .filter(|m| m.realm_id == attempt.realm_id)
            .ok_or_else(|| {
                CoreError::not_found(&format!(
                    "Entitlement mapping {} for one-time purchase",
                    attempt.target_id
                ))
            })?;

        // Read points_per_period from mapping for points count
        let points = mapping.points_per_period.ok_or_else(|| {
            CoreError::InternalServerError(format!(
                "Entitlement mapping '{}' has no points_per_period configured",
                mapping.entitlement_key
            ))
        })?;

        // Calculate expiration from validity_days
        let expires_at = mapping
            .validity_days
            .map(|days| chrono::Utc::now() + chrono::Duration::days(days));

        // Grant TopupCredit via points_repository
        // Use attempt.id as source_id AND idempotency_key to prevent double-grant on concurrent webhooks
        // A8: route grant to `attempt.bucket_id` snapshot (source of truth). Live
        // mapping.bucket_id is not consulted.
        let bucket_id = attempt.bucket_id;
        let credit_ledger = self
            .points_repository
            .grant_points_atomic(
                &attempt.realm_id,
                attempt.user_id,
                bucket_id,
                CreditType::TopupCredit,
                CreditSourceType::Topup,
                points,
                expires_at,
                Some(attempt.id.to_string()),
                Some(format!(
                    "One-time purchase: {} ({} points) via {}",
                    mapping.entitlement_key, points, provider_transaction_id
                )),
                Some(format!("one_time_purchase:{}", attempt.id)),
            )
            .await?;

        tracing::info!(
            payment_attempt_id = %attempt.id,
            user_id = %attempt.user_id,
            points,
            entitlement_key = %mapping.entitlement_key,
            "One-time purchase fulfilled, topup_credit granted"
        );

        let points_grant = PointsGrant {
            transaction_id: credit_ledger.id,
            points_type: "topup_credit".to_string(),
            points,
            description: format!(
                "One-time purchase: {} ({} points) via {}",
                mapping.entitlement_key, points, provider_transaction_id
            ),
        };

        Ok(FulfillmentResult {
            fulfillment_type: FulfillmentType::PointsGranted,
            subscription_id: None,
            points_granted: Some(points_grant),
            granted_at: chrono::Utc::now(),
        })
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_billing_period_to_days() {
        assert_eq!(billing_period_to_days(Some("daily")), 1);
        assert_eq!(billing_period_to_days(Some("day")), 1);
        assert_eq!(billing_period_to_days(Some("weekly")), 7);
        assert_eq!(billing_period_to_days(Some("month")), 30);
        assert_eq!(billing_period_to_days(Some("yearly")), 365);
        assert_eq!(billing_period_to_days(None), 30);
    }
}
