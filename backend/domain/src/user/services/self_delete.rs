// Account self-deletion (soft-delete) service — BE-D07.
//
// Implements the design §5.2 self-service deletion pipeline:
//   pre-tx : bcrypt verify password, status guard, cancel active subscriptions
//   in-tx  : account anonymization
//            (email/password/username/provider_ids), status=Deleted, profile
//            nickname clear, TOTP config delete — all delegated to the
//            `UserRepository::anonymize_user_for_deletion` port so the
//            anonymization is one atomic DB transaction.
//   post-tx: delete all user
//            sessions (incl. caller's, required), write `user.delete` audit with
//            `details.method = self_service` (best-effort).
//
// The service holds the concrete repository types rather than `Arc<dyn>`,
// mirroring `LegalService` (the port traits return `impl Future` and are
// therefore not object-safe).

use std::sync::Arc;

use uuid::Uuid;

use crate::audit::{
    ActorType, AuditAction, AuditCategory, AuditEventRepository, AuditResult, AuditTargetType,
    NewAuditEvent,
};
use crate::authentication::Identity;
use crate::authentication::ports::SessionRepository;
use crate::billing::ports::BillingRepository;
use crate::common::entities::app_errors::CoreError;
use crate::user::entities::UserStatus;
use crate::user::ports::UserRepository;
use crate::user_totp::ports::UserTotpRepository;

pub struct SelfDeleteService<U, T, B, S, A>
where
    U: UserRepository,
    T: UserTotpRepository,
    B: BillingRepository,
    S: SessionRepository,
    A: AuditEventRepository,
{
    user_repo: Arc<U>,
    // Kept for future direct TOTP queries; the in-tx TOTP wipe happens inside
    // `anonymize_user_for_deletion` for atomicity (single transaction).
    #[allow(dead_code)]
    totp_repo: Arc<T>,
    billing_repo: Arc<B>,
    session_repo: Arc<S>,
    audit_repo: Arc<A>,
}

impl<U, T, B, S, A> SelfDeleteService<U, T, B, S, A>
where
    U: UserRepository,
    T: UserTotpRepository,
    B: BillingRepository,
    S: SessionRepository,
    A: AuditEventRepository,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repo: Arc<U>,
        totp_repo: Arc<T>,
        billing_repo: Arc<B>,
        session_repo: Arc<S>,
        audit_repo: Arc<A>,
    ) -> Self {
        Self {
            user_repo,
            totp_repo,
            billing_repo,
            session_repo,
            audit_repo,
        }
    }

    /// Self-service account deletion.
    ///
    /// Identity is the authenticated caller; `password` is the second-factor
    /// confirmation. Errors:
    ///   - [`CoreError::NotFound`] — account does not exist (should not happen
    ///     for a valid session).
    ///   - [`CoreError::Unauthorized`] — password missing / wrong.
    ///   - [`CoreError::Conflict`] — account is already in the `Deleted` state.
    pub async fn self_delete_account(
        &self,
        identity: &Identity,
        password: &str,
    ) -> Result<(), CoreError> {
        let user_id = parse_user_id(identity.user_id())?;
        let realm_id = identity.realm_id();

        // ---- Phase 1: load + verify + status guard ----
        let account = self.user_repo.get_user_by_id(user_id).await?;

        // Status guard: an already-Deleted account is an idempotent repeat — 409.
        if account.status == UserStatus::Deleted {
            return Err(CoreError::Conflict(
                "Account is already deleted".to_string(),
            ));
        }

        // Password second-factor: missing hash or wrong password => 401.
        // Reuse the domain `verify_password` helper (handles `None` as false).
        let password_ok = account.verify_password(password).map_err(|_| {
            CoreError::InternalServerError("Password verification failed".to_string())
        })?;
        if !password_ok {
            return Err(CoreError::Unauthorized);
        }

        // ---- Phase 2: cancel active subscriptions before deletion ----
        // A cancellation failure leaves the account reachable so the user can
        // retry instead of losing access while billing may continue.
        let active_subs = self
            .billing_repo
            .list_active_subscriptions_by_user(&realm_id, user_id)
            .await?;
        for sub in active_subs {
            self.billing_repo.cancel_subscription(sub.id, false).await?;
        }

        // ---- Phase 3: in-tx anonymization (account + profile + totp_config) ----
        // A single repository transaction. The anonymized email is derived from
        // the account id so it is unique within `(realm_id, email)`.
        self.user_repo
            .anonymize_user_for_deletion(&realm_id, user_id)
            .await?;

        // ---- Phase 4: post-tx side effects ----
        // Session revocation is required so a deleted account cannot keep
        // using protected endpoints. Audit remains best-effort.

        // 4a. Revoke all of the user's sessions / tokens (incl. the caller's
        //     current session — delete == logout).
        let user_id_str = user_id.to_string();
        self.session_repo
            .delete_user_sessions(&user_id_str)
            .await
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    user_id = %user_id,
                    "self_delete: failed to delete user sessions"
                );
                e
            })?;

        // 4b. Audit `user.delete` (Compliance, method=self_service). Reuses the
        //     existing UserDelete action (slug `user.delete`) — no near-synonym.
        let audit_event = NewAuditEvent {
            realm_id: realm_id.clone(),
            category: AuditCategory::Compliance,
            action: AuditAction::UserDelete,
            actor_id: user_id.to_string(),
            actor_type: Some(ActorType::User),
            actor_name: None,
            target_type: AuditTargetType::User,
            target_id: user_id.to_string(),
            target_name: None,
            result: AuditResult::Success,
            details: Some(serde_json::json!({
                "method": "self_service",
                "anonymized": true,
            })),
            ip_address: None,
            user_agent: None,
            trace_id: None,
        };
        if let Err(e) = self.audit_repo.create(audit_event).await {
            tracing::warn!(
                error = %e,
                user_id = %user_id,
                "self_delete: failed to write audit event; continuing"
            );
        }

        Ok(())
    }
}

impl<U, T, B, S, A> std::fmt::Debug for SelfDeleteService<U, T, B, S, A>
where
    U: UserRepository,
    T: UserTotpRepository,
    B: BillingRepository,
    S: SessionRepository,
    A: AuditEventRepository,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelfDeleteService").finish()
    }
}

/// `identity.user_id()` is a `String`; the user repo takes a `Uuid`. A
/// malformed id means the identity was constructed without a valid user row —
/// surface as 500.
fn parse_user_id(raw: String) -> Result<Uuid, CoreError> {
    Uuid::parse_str(&raw).map_err(|e| {
        CoreError::InternalServerError(format!("identity user_id is not a valid uuid: {e}"))
    })
}
