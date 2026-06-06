// Purchase fulfillment service implementation

use std::sync::Arc;

use herald_domain::billing::{BillingRepository, Subscription, SubscriptionStatus};
use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::payment_attempt::PaymentAttempt;
use herald_domain::points::{CreditSourceType, CreditType, PointsRepository};
use herald_domain::points_package::PointsPackageRepository;
use herald_domain::purchase::{
    CreatePointsPackagePurchaseInput, FulfillmentResult, FulfillmentService, FulfillmentType,
    PointsGrant, PurchaseRepository,
};

/// Helper function to detect duplicate key errors (unique constraint violations)
fn is_duplicate_key_error(err: &CoreError) -> bool {
    if let CoreError::DatabaseError(msg) = err {
        let msg_lower = msg.to_lowercase();
        msg_lower.contains("duplicate key") || msg_lower.contains("unique constraint")
    } else {
        false
    }
}
/// Implementation of fulfillment service for unified purchase handling
pub struct PostgresFulfillmentService<P, PP, PR, B>
where
    P: PointsRepository,
    PP: PointsPackageRepository,
    PR: PurchaseRepository,
    B: BillingRepository,
{
    points_repository: Arc<P>,
    points_package_repository: Arc<PP>,
    purchase_repository: Arc<PR>,
    billing_repository: Arc<B>,
}

impl<P, PP, PR, B> PostgresFulfillmentService<P, PP, PR, B>
where
    P: PointsRepository,
    PP: PointsPackageRepository,
    PR: PurchaseRepository,
    B: BillingRepository,
{
    pub fn new(
        points_repository: Arc<P>,
        points_package_repository: Arc<PP>,
        purchase_repository: Arc<PR>,
        billing_repository: Arc<B>,
    ) -> Self {
        Self {
            points_repository,
            points_package_repository,
            purchase_repository,
            billing_repository,
        }
    }
}

impl<P, PP, PR, B> FulfillmentService for PostgresFulfillmentService<P, PP, PR, B>
where
    P: PointsRepository + Send + Sync,
    PP: PointsPackageRepository + Send + Sync,
    PR: PurchaseRepository + Send + Sync,
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

        // Look up entitlement mapping by external_product_id (= target_id)
        let mapping = self
            .billing_repository
            .find_entitlement_mapping_by_provider_product(
                &attempt.realm_id,
                &attempt.payment_provider,
                &attempt.target_id.to_string(),
            )
            .await?
            .ok_or_else(|| {
                CoreError::BadRequest(format!(
                    "No entitlement mapping found for provider '{}' product '{}' in realm '{}'",
                    attempt.payment_provider, attempt.target_id, attempt.realm_id
                ))
            })?;

        let entitlement_key = mapping.entitlement_key.clone();

        let now = chrono::Utc::now();
        let period_end = now + chrono::Duration::days(30); // Default 30-day period

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

        Ok(FulfillmentResult {
            fulfillment_type: FulfillmentType::SubscriptionCreated,
            subscription_id: Some(created_subscription.id),
            points_granted: None,
            granted_at: created_subscription.created_at,
        })
    }

    async fn fulfill_points_package_purchase(
        &self,
        attempt: &PaymentAttempt,
        provider_transaction_id: String,
    ) -> Result<FulfillmentResult, CoreError> {
        // Check for existing fulfillment (idempotency check - PART 1: purchase record)
        if let Some(existing_purchase) = self
            .purchase_repository
            .find_points_package_purchase_by_attempt_id(attempt.id)
            .await?
            && let Some(transaction_id) = existing_purchase.points_transaction_id
        {
            return Ok(FulfillmentResult {
                fulfillment_type: FulfillmentType::PointsGranted,
                subscription_id: None,
                points_granted: Some(PointsGrant {
                    transaction_id,
                    points_type: "topup_credit".to_string(),
                    points: existing_purchase.points,
                    description: format!(
                        "Purchased points package {} (Payment: {})",
                        existing_purchase.points_package_id, provider_transaction_id
                    ),
                }),
                granted_at: existing_purchase.updated_at,
            });
        }

        // Check for existing fulfillment (idempotency check - PART 2: credit ledger)
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
                        "Purchased points package (Payment: {})",
                        provider_transaction_id
                    ),
                }),
                granted_at: existing_ledger.created_at,
            });
        }

        // Fetch the points package to get the amount
        let package = self
            .points_package_repository
            .find_points_package_by_id(&attempt.realm_id, attempt.target_id)
            .await?
            .ok_or_else(|| {
                CoreError::not_found(&format!("Points package {}", attempt.target_id))
            })?;

        // Grant topup_credit points to the user
        let credit_ledger = self
            .points_repository
            .grant_points_atomic(
                &attempt.realm_id,
                attempt.user_id,
                CreditType::TopupCredit,
                CreditSourceType::Topup,
                package.points,
                None, // No expiration for purchased points
                Some(attempt.id.to_string()),
                None, // description
            )
            .await?;

        // Create purchase record - this may fail with unique constraint violation
        // if another concurrent request already fulfilled this payment attempt
        let purchase_result = self
            .purchase_repository
            .create_points_package_purchase(CreatePointsPackagePurchaseInput {
                realm_id: attempt.realm_id.clone(),
                user_id: attempt.user_id,
                points_package_id: package.id,
                payment_attempt_id: attempt.id,
                points: package.points,
                amount: attempt.amount,
                currency: attempt.currency.clone(),
                payment_provider: attempt.payment_provider.clone(),
            })
            .await;

        let purchase = match purchase_result {
            Ok(purchase) => purchase,
            Err(e) if is_duplicate_key_error(&e) => {
                tracing::info!(
                    payment_attempt_id = %attempt.id,
                    "Concurrent fulfillment detected, returning existing purchase"
                );
                let existing_purchase = self
                    .purchase_repository
                    .find_points_package_purchase_by_attempt_id(attempt.id)
                    .await?
                    .ok_or_else(|| {
                        CoreError::InternalServerError(
                            "Purchase record missing after unique constraint violation".to_string(),
                        )
                    })?;

                if let Some(transaction_id) = existing_purchase.points_transaction_id {
                    return Ok(FulfillmentResult {
                        fulfillment_type: FulfillmentType::PointsGranted,
                        subscription_id: None,
                        points_granted: Some(PointsGrant {
                            transaction_id,
                            points_type: "topup_credit".to_string(),
                            points: existing_purchase.points,
                            description: format!(
                                "Purchased points package {} (Payment: {})",
                                existing_purchase.points_package_id, provider_transaction_id
                            ),
                        }),
                        granted_at: existing_purchase.updated_at,
                    });
                }

                self.purchase_repository
                    .update_purchase_transaction_id(existing_purchase.id, credit_ledger.id)
                    .await?;

                return Ok(FulfillmentResult {
                    fulfillment_type: FulfillmentType::PointsGranted,
                    subscription_id: None,
                    points_granted: Some(PointsGrant {
                        transaction_id: credit_ledger.id,
                        points_type: "topup_credit".to_string(),
                        points: package.points,
                        description: format!(
                            "Purchased points package: {} (Payment: {})",
                            package.title, provider_transaction_id
                        ),
                    }),
                    granted_at: chrono::Utc::now(),
                });
            }
            Err(e) => return Err(e),
        };

        self.purchase_repository
            .update_purchase_transaction_id(purchase.id, credit_ledger.id)
            .await?;

        let points_grant = PointsGrant {
            transaction_id: credit_ledger.id,
            points_type: "topup_credit".to_string(),
            points: package.points,
            description: format!(
                "Purchased points package: {} (Payment: {})",
                package.title, provider_transaction_id
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
    fn test_is_duplicate_key_error() {
        let duplicate_error = CoreError::DatabaseError(
            "duplicate key value violates unique constraint \"uq_points_package_purchases_payment_attempt\"".to_string(),
        );
        assert!(is_duplicate_key_error(&duplicate_error));

        let unique_constraint_error =
            CoreError::DatabaseError("unique constraint violation".to_string());
        assert!(is_duplicate_key_error(&unique_constraint_error));

        let other_error = CoreError::NotFound;
        assert!(!is_duplicate_key_error(&other_error));

        let db_error = CoreError::DatabaseError("connection failed".to_string());
        assert!(!is_duplicate_key_error(&db_error));
    }
}
