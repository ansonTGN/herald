// Points Service - Business logic for points operations

use std::sync::Arc;
use uuid::Uuid;

use crate::authentication::Identity;
use crate::common::entities::app_errors::CoreError;
use crate::common::policies::ensure_policy;
use crate::points::{
    dtos::{ConsumePointsInput, GrantPointsInput, GrantPointsOutput, RevokePointsOutput},
    entities::{
        CreditSourceType, CreditType, Paginated, PointsBalance, PointsTransaction, PointsWallet,
        RechargeType, RevocationType, WalletStatus,
    },
    errors::PointsErrorExt,
    policies::PointsPolicy,
    ports::{PointsRepository, TransactionFilters, WalletFilters},
};

/// Points Service - Business logic for points management
///
/// Includes permission-based authorization checks using PointsPolicy
pub struct PointsService<R, P>
where
    R: PointsRepository,
    P: PointsPolicy,
{
    repository: Arc<R>,
    policy: Arc<P>,
}

impl<R, P> PointsService<R, P>
where
    R: PointsRepository + Send + Sync,
    P: PointsPolicy,
{
    pub fn new(repository: Arc<R>, policy: Arc<P>) -> Self {
        Self { repository, policy }
    }

    // ===== Account Management =====

    /// Get points wallet for a user
    pub async fn get_wallet(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<PointsWallet, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy
                .can_view_points(identity.clone(), Some(user_id))
                .await,
            "Insufficient permissions to view points wallet",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access points from a different realm".to_string(),
            ));
        }

        // Find or create account
        match self.repository.find_by_user_id(realm_id, user_id).await? {
            Some(account) => Ok(account),
            None => {
                // Auto-create account if it doesn't exist
                tracing::info!("Auto-creating points wallet for user {}", user_id);
                let new_account = self.create_wallet_internal(realm_id, user_id).await?;
                Ok(new_account)
            }
        }
    }

    /// Get points balance for a user
    pub async fn get_balance(
        &self,
        identity: Identity,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<PointsBalance, CoreError> {
        let account = self.get_wallet(identity, realm_id, user_id).await?;
        Ok(account.into())
    }

    /// List all accounts in a realm (admin only)
    pub async fn list_wallets(
        &self,
        identity: Identity,
        realm_id: &str,
        filters: WalletFilters,
    ) -> Result<Paginated<PointsWallet>, CoreError> {
        // Check manage permissions
        ensure_policy(
            self.policy.can_manage_points(identity.clone()).await,
            "Insufficient permissions to list wallets",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access points from a different realm".to_string(),
            ));
        }

        self.repository.list_wallets(realm_id, filters).await
    }

    // ===== Helper Methods =====

    // ===== Points Consumption =====

    /// Consume points from a user's account using ledger-based consumption
    ///
    /// Consumption priority: expiration-based (soonest expiring first, permanent last)
    /// This is the NEW correct implementation per the design specification.
    pub async fn consume_points(
        &self,
        identity: Identity,
        realm_id: &str,
        input: ConsumePointsInput,
    ) -> Result<PointsTransaction, CoreError> {
        // Check consume permissions
        ensure_policy(
            self.policy.can_consume_points(identity.clone()).await,
            "Insufficient permissions to consume points",
        )?;

        // Validate input
        let (user_id, client_app_id, amount, description) = input.try_into()?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot consume points from a different realm".to_string(),
            ));
        }

        // Get or create account
        let account = match self.repository.find_by_user_id(realm_id, user_id).await? {
            Some(account) => account,
            None => {
                // Auto-create account
                self.create_wallet_internal(realm_id, user_id).await?
            }
        };

        // Check wallet status
        if account.status != WalletStatus::Active {
            return Err(CoreError::BadRequest(format!(
                "Cannot consume points from {} wallet",
                account.status.as_str()
            )));
        }

        // Check total balance
        if account.total_balance < amount {
            return Err(CoreError::insufficient_points(
                amount,
                account.total_balance,
            ));
        }

        let saved_transaction = self
            .repository
            .consume_points_atomic(realm_id, user_id, client_app_id, amount, description)
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            amount,
            balance_after = %saved_transaction.balance_after,
            "Points consumed successfully (expiration-based priority)"
        );

        Ok(saved_transaction)
    }

    // ===== Transaction Management =====

    /// Get a single transaction by ID
    pub async fn get_transaction(
        &self,
        identity: Identity,
        realm_id: &str,
        transaction_id: Uuid,
    ) -> Result<PointsTransaction, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy.can_view_points(identity.clone(), None).await,
            "Insufficient permissions to view transaction",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access transaction from a different realm".to_string(),
            ));
        }

        let transaction = self
            .repository
            .find_transaction_by_id(realm_id, transaction_id)
            .await?
            .ok_or(CoreError::NotFound)?;

        Ok(transaction)
    }

    /// List transactions with filters
    pub async fn list_transactions(
        &self,
        identity: Identity,
        realm_id: &str,
        filters: TransactionFilters,
    ) -> Result<Paginated<PointsTransaction>, CoreError> {
        // Check view permissions
        ensure_policy(
            self.policy
                .can_view_points(identity.clone(), filters.user_id)
                .await,
            "Insufficient permissions to view transactions",
        )?;

        // Check realm boundary
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot access transactions from a different realm".to_string(),
            ));
        }

        // Only points managers can query across users; points.view alone is scoped to self.
        let can_view_all = self.policy.can_manage_points(identity.clone()).await;
        if !can_view_all && let Ok(current_user_id) = identity.user_id().parse::<Uuid>() {
            // Override filters to only show current user's transactions
            let mut restricted_filters = filters;
            restricted_filters.user_id = Some(current_user_id);
            return self
                .repository
                .find_transactions(realm_id, restricted_filters)
                .await;
        }

        self.repository.find_transactions(realm_id, filters).await
    }

    // ===== Points Grant =====

    /// Grant points to a user (admin endpoint)
    ///
    /// Performs permission check and realm boundary check, validates input,
    /// then delegates to `grant_points_internal`.
    pub async fn grant_points(
        &self,
        identity: Identity,
        realm_id: &str,
        input: GrantPointsInput,
    ) -> Result<GrantPointsOutput, CoreError> {
        // Permission check
        ensure_policy(
            self.policy.can_manage_points(identity.clone()).await,
            "Insufficient permissions to grant points",
        )?;

        // Realm boundary check
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cannot grant points from a different realm".to_string(),
            ));
        }

        self.execute_grant(realm_id, input).await
    }

    /// Grant points to a user (SDK/ext endpoint)
    ///
    /// Skips identity/policy checks -- those are handled by the caller
    /// at the middleware level (API Key authentication).
    pub async fn grant_points_for_sdk(
        &self,
        realm_id: &str,
        input: GrantPointsInput,
    ) -> Result<GrantPointsOutput, CoreError> {
        self.execute_grant(realm_id, input).await
    }

    /// Shared implementation for granting points
    async fn execute_grant(
        &self,
        realm_id: &str,
        input: GrantPointsInput,
    ) -> Result<GrantPointsOutput, CoreError> {
        // Validate input
        input.validate()?;

        // Compute expires_at from validity_days
        let expires_at = input
            .validity_days
            .map(|days| chrono::Utc::now() + chrono::Duration::days(days));

        // Build description including the user-provided reason
        let description = Some(format!(
            "{}: {} points granted ({})",
            input.source_type.as_str(),
            input.amount,
            input.reason
        ));

        // Grant points via internal method
        let ledger_id = self
            .grant_points_internal(
                realm_id,
                input.user_id,
                CreditType::GrantedCredit,
                input.source_type,
                input.amount,
                expires_at,
                Some(input.source_id),
                description,
            )
            .await?;

        // Fetch the updated wallet to get current balances
        let wallet = self
            .repository
            .find_by_user_id(realm_id, input.user_id)
            .await?
            .ok_or_else(|| CoreError::wallet_not_found(&input.user_id.to_string()))?;

        Ok(GrantPointsOutput {
            transaction_id: ledger_id,
            user_id: input.user_id,
            amount: input.amount,
            granted_balance: wallet.granted_balance,
            total_balance: wallet.total_balance,
            expires_at,
        })
    }

    // ===== Internal Methods =====

    /// Create a new points wallet (internal use)
    async fn create_wallet_internal(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<PointsWallet, CoreError> {
        let account = PointsWallet {
            id: Uuid::now_v7(),
            user_id,
            realm_id: realm_id.to_string(),
            total_balance: 0,
            topup_balance: 0,
            subscription_balance: 0,
            granted_balance: 0,
            registration_balance: 0,
            free_periodic_balance: 0,
            total_topup_granted: 0,
            total_subscription_granted: 0,
            total_recharged: 0,
            total_consumed: 0,
            status: WalletStatus::Active,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.repository.create_wallet(account).await
    }

    /// Recharge points for a user (internal method for billing webhooks)
    pub async fn recharge_points_internal(
        &self,
        realm_id: &str,
        user_id: Uuid,
        amount: i64,
        recharge_type: RechargeType,
        external_ref_id: Option<String>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<PointsTransaction, CoreError> {
        if amount <= 0 {
            return Err(CoreError::invalid_amount(
                "Recharge amount must be positive",
            ));
        }

        // Check wallet status before recharging
        let account = self.repository.find_by_user_id(realm_id, user_id).await?;
        if let Some(account) = &account
            && account.status != WalletStatus::Active
        {
            return Err(CoreError::BadRequest(format!(
                "Cannot recharge points to {} wallet",
                account.status.as_str()
            )));
        }

        let (credit_type, source_type) = match recharge_type {
            RechargeType::Subscribe => (
                CreditType::SubscriptionCredit,
                CreditSourceType::SubscriptionInitial,
            ),
            RechargeType::Renewal => (
                CreditType::SubscriptionCredit,
                CreditSourceType::SubscriptionRenewal,
            ),
        };

        let transaction = self
            .repository
            .recharge_points_atomic(
                realm_id,
                user_id,
                credit_type,
                source_type,
                amount,
                expires_at,
                None,
                external_ref_id,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            amount,
            recharge_type = %recharge_type.as_str(),
            balance_after = %transaction.balance_after,
            "Points recharged successfully"
        );

        Ok(transaction)
    }

    /// Revoke points by credit type (internal method for subscription cancellation and refunds)
    ///
    /// Revokes all unused points of a specific credit type for a user.
    /// This is used for subscription cancellation, refunds, and expiration.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `credit_type` - The type of credit to revoke (topup or subscription)
    /// * `revocation_type` - The reason for revocation
    /// * `reason` - Human-readable reason
    ///
    /// # Returns
    /// Revocation output with details of revoked points
    pub async fn revoke_points_by_credit_type(
        &self,
        realm_id: &str,
        user_id: Uuid,
        credit_type: CreditType,
        revocation_type: RevocationType,
        reason: String,
    ) -> Result<RevokePointsOutput, CoreError> {
        let result = self
            .repository
            .revoke_points_by_credit_type_atomic(
                realm_id,
                user_id,
                credit_type,
                revocation_type,
                reason,
                None,
                None,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            credit_type = %credit_type.as_str(),
            total_revoked = result.total_revoked,
            ledger_count = result.ledger_ids.len(),
            revocation_type = %revocation_type.as_str(),
            "Points revoked successfully"
        );

        Ok(result)
    }

    /// Revoke remaining points from a specific ledger identified by source_id.
    /// Unlike `revoke_points_by_credit_type`, this targets only the single ledger
    /// whose `source_id` matches, preventing over-broad revocation of unrelated credits.
    pub async fn revoke_points_by_source_id(
        &self,
        realm_id: &str,
        user_id: Uuid,
        source_id: &str,
        revocation_type: RevocationType,
        reason: String,
    ) -> Result<RevokePointsOutput, CoreError> {
        let result = self
            .repository
            .revoke_points_by_source_id_atomic(
                realm_id,
                user_id,
                source_id,
                revocation_type,
                reason,
                None,
                None,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            source_id = %source_id,
            total_revoked = result.total_revoked,
            revocation_type = %revocation_type.as_str(),
            "Points revoked by source_id successfully"
        );

        Ok(result)
    }

    /// Revoke all daily free credits for a user (used when free user upgrades to paid)
    ///
    /// **Idempotency Guarantee**:
    /// - If idempotency_key is provided, checks if already processed
    /// - If no active daily credits exist, returns empty result (success)
    /// - Creates idempotency record even when no credits are revoked
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `reason` - Reason for revocation
    /// * `idempotency_key` - Optional idempotency key for deduplication
    ///
    /// # Returns
    /// Revocation output with details of revoked credits
    pub async fn revoke_all_daily_credits(
        &self,
        realm_id: &str,
        user_id: Uuid,
        reason: String,
        idempotency_key: Option<String>,
    ) -> Result<RevokePointsOutput, CoreError> {
        self.repository
            .revoke_points_by_credit_type_atomic(
                realm_id,
                user_id,
                CreditType::FreePeriodicCredit,
                RevocationType::UpgradeRevoke,
                reason,
                None,
                idempotency_key,
            )
            .await
    }

    /// Proportionally revoke topup points based on refund ratio
    ///
    /// When a topup payment is refunded, we need to revoke the proportionate amount
    /// of points based on the refund ratio. This ensures users don't keep points they didn't pay for.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `refund_amount` - The amount being refunded (in cents)
    /// * `original_payment_amount` - The original payment amount (in cents)
    /// * `refund_id` - The refund ID for reference
    ///
    /// # Returns
    /// Revocation output with details of revoked points
    pub async fn revoke_topup_proportional(
        &self,
        realm_id: &str,
        user_id: Uuid,
        refund_amount: i64,
        original_payment_amount: i64,
        refund_id: &str,
    ) -> Result<RevokePointsOutput, CoreError> {
        if original_payment_amount <= 0 {
            return Err(CoreError::BadRequest(
                "Original payment amount must be positive".to_string(),
            ));
        }

        if refund_amount <= 0 {
            return Err(CoreError::BadRequest(
                "Refund amount must be positive".to_string(),
            ));
        }

        if refund_amount > original_payment_amount {
            return Err(CoreError::BadRequest(
                "Refund amount cannot exceed original payment".to_string(),
            ));
        }

        // Calculate refund ratio
        let refund_ratio = (refund_amount as f64) / (original_payment_amount as f64);

        let result = self
            .repository
            .revoke_topup_proportional_atomic(realm_id, user_id, refund_ratio, refund_id)
            .await?;

        if result.total_revoked == 0 {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                refund_id = %refund_id,
                "No active topup ledgers found for proportional revocation"
            );
        } else {
            tracing::info!(
                realm_id = %realm_id,
                user_id = %user_id,
                refund_id = %refund_id,
                refund_ratio = refund_ratio,
                total_revoked = result.total_revoked,
                ledger_count = result.ledger_ids.len(),
                "Proportionally revoked topup points"
            );
        }

        Ok(result)
    }

    /// Revoke all unused subscription points
    ///
    /// This is a convenience method for refund scenarios where all subscription
    /// points need to be revoked. It simply calls revoke_points_by_credit_type
    /// with SubscriptionCredit and RefundRevoke type.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `refund_id` - The refund ID for reference
    ///
    /// # Returns
    /// Revocation output with details of revoked points
    pub async fn revoke_subscription_unused(
        &self,
        realm_id: &str,
        user_id: Uuid,
        refund_id: &str,
    ) -> Result<RevokePointsOutput, CoreError> {
        self.revoke_points_by_credit_type(
            realm_id,
            user_id,
            CreditType::SubscriptionCredit,
            crate::points::entities::RevocationType::RefundRevoke,
            format!("Refund {}", refund_id),
        )
        .await
    }

    /// Refund points from a user's account
    ///
    /// Creates a refund transaction to deduct points when a subscription is refunded.
    /// This is typically called when a payment refund is processed.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `subscription_id` - The subscription ID being refunded
    /// * `refund_amount` - The amount of points to refund (deduct)
    /// * `reason` - The reason for the refund
    ///
    /// # Returns
    /// The created refund transaction
    ///
    /// # Errors
    /// - Account not found
    /// - Invalid amount (must be positive)
    /// - Database errors
    pub async fn refund_points(
        &self,
        realm_id: &str,
        user_id: Uuid,
        subscription_id: String,
        refund_amount: i64,
        reason: String,
    ) -> Result<PointsTransaction, CoreError> {
        if refund_amount <= 0 {
            return Err(CoreError::BadRequest(
                "Refund amount must be positive".to_string(),
            ));
        }

        let created_transaction = self
            .repository
            .refund_points_atomic(
                realm_id,
                user_id,
                subscription_id.clone(),
                refund_amount,
                reason,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            subscription_id = %subscription_id,
            refund_amount,
            new_balance = created_transaction.balance_after,
            "Points refunded successfully"
        );

        Ok(created_transaction)
    }

    // ===== Internal Methods (for Background Services) =====

    /// Internal method to grant points directly to ledger
    ///
    /// This is used by background services (registration, scheduler)
    /// and bypasses the public API layer validation.
    ///
    /// # Arguments
    /// * `realm_id` - The realm ID
    /// * `user_id` - The user ID
    /// * `credit_type` - Type of credit to grant
    /// * `source_type` - Source of the grant
    /// * `amount` - Amount to grant
    /// * `expires_at` - Optional expiration time (None = permanent)
    /// * `source_id` - Optional source ID for traceability
    ///
    /// # Returns
    /// Ok(ledger_id) on success -- the ID of the created credit ledger entry
    ///
    /// # Errors
    /// - InvalidAmount if amount <= 0
    /// - Database errors
    ///
    /// # Security
    /// This is an internal method (NOT an HTTP endpoint) and will only be called
    /// from trusted internal services (RegistrationService, GrantScheduler).
    /// No authorization checks are performed.
    pub async fn grant_points_internal(
        &self,
        realm_id: &str,
        user_id: Uuid,
        credit_type: CreditType,
        source_type: crate::points::entities::CreditSourceType,
        amount: i64,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        source_id: Option<String>,
        description: Option<String>,
    ) -> Result<Uuid, CoreError> {
        if amount <= 0 {
            return Err(CoreError::BadRequest(
                "Grant amount must be positive".to_string(),
            ));
        }

        // Check wallet status before granting
        let account = self.repository.find_by_user_id(realm_id, user_id).await?;
        if let Some(account) = &account
            && account.status != WalletStatus::Active
        {
            return Err(CoreError::BadRequest(format!(
                "Cannot grant points to {} wallet",
                account.status.as_str()
            )));
        }

        let saved_ledger = self
            .repository
            .grant_points_atomic(
                realm_id,
                user_id,
                credit_type,
                source_type,
                amount,
                expires_at,
                source_id,
                description,
            )
            .await?;

        tracing::info!(
            realm_id = %realm_id,
            user_id = %user_id,
            amount,
            credit_type = ?credit_type,
            source_type = ?source_type,
            expires_at = ?expires_at,
            "Points granted internally"
        );

        Ok(saved_ledger.id)
    }
}
