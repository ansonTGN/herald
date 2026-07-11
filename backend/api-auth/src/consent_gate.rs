use herald_api_base::application::http::state::AppState;
use herald_core::domain::audit::ActorType;
use herald_core::domain::legal::{
    AgreementType, AuditActorMeta, ConsentSource, LegalAgreementSummary,
};
use herald_core::domain::user::entities::User;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuthConsentAgreement {
    pub agreement_type: String,
    pub version_id: Uuid,
}

pub async fn evaluate_login_consent_gate(
    state: &AppState,
    user: &User,
    realm_id: &str,
    accepted_agreements: Option<&[AuthConsentAgreement]>,
    ip_address: Option<String>,
    user_agent: Option<String>,
) -> Option<Vec<LegalAgreementSummary>> {
    let actor_meta = AuditActorMeta {
        actor_id: user.id.to_string(),
        actor_type: ActorType::User,
        actor_name: Some(user.email.clone()),
        ip_address,
        user_agent,
        trace_id: None,
    };

    let status_items = match state.legal_service.consent_status(user.id, realm_id).await {
        Ok(items) => items,
        Err(e) => {
            tracing::warn!(
                user_id = %user.id,
                realm_id = %realm_id,
                error = %e,
                "consent_status lookup failed; skipping consent gate (fail-open)"
            );
            Vec::new()
        }
    };

    let needs_reconsent = status_items.iter().any(|i| i.needs_reconsent);
    if needs_reconsent {
        let mut summaries = Vec::with_capacity(status_items.len());
        for item in &status_items {
            if let Ok(Some(version)) = state
                .legal_service
                .current_effective(realm_id, item.agreement_type.clone())
                .await
            {
                summaries.push(LegalAgreementSummary {
                    agreement_type: item.agreement_type.as_str().to_string(),
                    version_id: version.id,
                    version_no: version.version_no,
                    effective_at: version.published_at,
                    title: None,
                    summary: None,
                    mode: version.mode,
                    external_url: version.external_url,
                });
            }
        }

        if let Some(accepted_agreements) = accepted_agreements
            && !accepted_agreements.is_empty()
        {
            let mut record_items = Vec::with_capacity(accepted_agreements.len());
            for item in accepted_agreements {
                let Ok(agreement_type) = AgreementType::try_from(item.agreement_type.as_str())
                else {
                    tracing::warn!(
                        user_id = %user.id,
                        realm_id = %realm_id,
                        agreement_type = %item.agreement_type,
                        "Invalid agreement type in login re-consent payload"
                    );
                    return Some(summaries);
                };
                record_items.push((agreement_type, item.version_id));
            }

            match state
                .legal_service
                .record_consent(
                    user.id,
                    realm_id,
                    record_items,
                    ConsentSource::Reconsent,
                    actor_meta.clone(),
                )
                .await
            {
                Ok(()) => match state.legal_service.consent_status(user.id, realm_id).await {
                    Ok(items) if !items.iter().any(|i| i.needs_reconsent) => {
                        tracing::info!(
                            user_id = %user.id,
                            realm_id = %realm_id,
                            "Login re-consent recorded; continuing login"
                        );
                        return None;
                    }
                    Ok(_) => {
                        tracing::warn!(
                            user_id = %user.id,
                            realm_id = %realm_id,
                            "Login re-consent payload did not satisfy all current agreements"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            user_id = %user.id,
                            realm_id = %realm_id,
                            error = %e,
                            "consent_status lookup failed after login re-consent"
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        user_id = %user.id,
                        realm_id = %realm_id,
                        error = %e,
                        "record_consent(Reconsent) failed during login"
                    );
                }
            }
        }

        tracing::info!(
            user_id = %user.id,
            realm_id = %realm_id,
            "Login blocked at consent gate (stale consent); returning consent_required"
        );

        return Some(summaries);
    }

    // A successful normal login is itself a consent event under the published
    // contract. Refresh the idempotent rows and preserve Login as the audit
    // source even when the accepted versions have not changed.
    let current_items = status_items
        .into_iter()
        .map(|item| (item.agreement_type, item.current_version_id))
        .collect();
    if let Err(error) = state
        .legal_service
        .record_consent(
            user.id,
            realm_id,
            current_items,
            ConsentSource::Login,
            actor_meta,
        )
        .await
    {
        tracing::warn!(
            user_id = %user.id,
            realm_id = %realm_id,
            error = %error,
            "record_consent(Login) failed during login"
        );
    }
    None
}
