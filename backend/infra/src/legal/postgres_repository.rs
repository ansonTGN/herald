use std::convert::TryFrom;

use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, RuntimeErr, Set,
};
use uuid::Uuid;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::legal::UserAgreementConsent;
use herald_domain::legal::entities::{AgreementSource, AgreementType, LegalAgreementVersion};
use herald_domain::legal::error::LegalError;
use herald_domain::legal::ports::{LegalAgreementRepository, UserConsentRepository};
use herald_entity::{legal_agreement_version, user_agreement_consent};

/// PostgreSQL implementation of [`LegalAgreementRepository`].
///
/// Holds a SeaORM `DatabaseConnection` (same constructor/shape as
/// `PostgresBillingRepository`). All row ↔ domain mapping happens here so the
/// service layer (BE-D04) only handles domain types.
pub struct PostgresLegalAgreementRepository {
    db: DatabaseConnection,
}

impl PostgresLegalAgreementRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Map a `legal_agreement_version` row to the domain entity.
    ///
    /// An `agreement_type` parse failure (a stray column value) is surfaced as
    /// `CoreError::InternalServerError` rather than silently corrupting
    /// resolution — the column is constrained to two values in practice, so a
    /// third means schema/operator drift that callers must not mask.
    fn to_domain(
        model: legal_agreement_version::Model,
    ) -> Result<LegalAgreementVersion, CoreError> {
        Ok(LegalAgreementVersion {
            id: model.id,
            realm_id: model.realm_id,
            agreement_type: AgreementType::try_from(model.agreement_type.as_str())
                .map_err(CoreError::InternalServerError)?,
            version_no: model.version_no,
            version_label: model.version_label,
            content: model.content,
            source: AgreementSource::from(model.source.as_str()),
            published_at: chrono::DateTime::<chrono::Utc>::from(model.published_at),
            published_by: model.published_by,
        })
    }

    /// Compute the next `version_no` for a custom (per-realm) publish:
    /// `max(version_no)` over the realm's custom rows + 1, or 1 when the realm
    /// has no custom version yet. Scoped to the realm's own rows only — the
    /// platform default rows (`realm_id IS NULL`) never participate, so a realm
    /// publishing for the first time starts at version_no = 1 alongside the
    /// default seed (they live in different scopes of the unique index).
    async fn next_custom_version_no(
        &self,
        realm_id: &str,
        agreement_type: &AgreementType,
    ) -> Result<i32, CoreError> {
        let row = legal_agreement_version::Entity::find()
            .filter(legal_agreement_version::Column::RealmId.eq(realm_id))
            .filter(legal_agreement_version::Column::AgreementType.eq(agreement_type.as_str()))
            .order_by_desc(legal_agreement_version::Column::VersionNo)
            .one(&self.db)
            .await?;
        Ok(row.map(|m| m.version_no + 1).unwrap_or(1))
    }
}

impl LegalAgreementRepository for PostgresLegalAgreementRepository {
    /// Effective resolution: latest realm-scoped custom row wins; if none
    /// exists, fall back to the latest platform-default row. Returns
    /// `Ok(None)` only when neither exists (caller decides 404 / deploy fault).
    async fn current_effective(
        &self,
        realm_id: &str,
        agreement_type: AgreementType,
    ) -> Result<Option<LegalAgreementVersion>, CoreError> {
        let custom = legal_agreement_version::Entity::find()
            .filter(legal_agreement_version::Column::RealmId.eq(realm_id))
            .filter(legal_agreement_version::Column::AgreementType.eq(agreement_type.as_str()))
            .order_by_desc(legal_agreement_version::Column::VersionNo)
            .one(&self.db)
            .await?;
        if let Some(model) = custom {
            return Self::to_domain(model).map(Some);
        }
        self.current_default(agreement_type).await
    }

    /// Latest platform-default row (`realm_id IS NULL`) for the type.
    async fn current_default(
        &self,
        agreement_type: AgreementType,
    ) -> Result<Option<LegalAgreementVersion>, CoreError> {
        let row = legal_agreement_version::Entity::find()
            .filter(legal_agreement_version::Column::RealmId.is_null())
            .filter(legal_agreement_version::Column::AgreementType.eq(agreement_type.as_str()))
            .order_by_desc(legal_agreement_version::Column::VersionNo)
            .one(&self.db)
            .await?;
        row.map(Self::to_domain).transpose()
    }

    /// History for the admin view: custom rows (version_no desc) first, then
    /// platform-default rows (version_no desc), truncated to `limit`. This
    /// ordering makes the realm's own evolution the dominant view and the
    /// platform baseline available as a trailing reference.
    async fn list_history(
        &self,
        realm_id: &str,
        agreement_type: AgreementType,
        limit: u64,
    ) -> Result<Vec<LegalAgreementVersion>, CoreError> {
        let custom_rows = legal_agreement_version::Entity::find()
            .filter(legal_agreement_version::Column::RealmId.eq(realm_id))
            .filter(legal_agreement_version::Column::AgreementType.eq(agreement_type.as_str()))
            .order_by_desc(legal_agreement_version::Column::VersionNo)
            .all(&self.db)
            .await?;
        let default_rows = legal_agreement_version::Entity::find()
            .filter(legal_agreement_version::Column::RealmId.is_null())
            .filter(legal_agreement_version::Column::AgreementType.eq(agreement_type.as_str()))
            .order_by_desc(legal_agreement_version::Column::VersionNo)
            .all(&self.db)
            .await?;

        let mut combined: Vec<LegalAgreementVersion> = Vec::with_capacity(custom_rows.len());
        for m in custom_rows {
            combined.push(Self::to_domain(m)?);
        }
        for m in default_rows {
            combined.push(Self::to_domain(m)?);
        }
        if combined.len() > limit as usize {
            combined.truncate(limit as usize);
        }
        Ok(combined)
    }

    /// Whether the realm has any custom version for the type.
    async fn has_custom(
        &self,
        realm_id: &str,
        agreement_type: AgreementType,
    ) -> Result<bool, CoreError> {
        let count = legal_agreement_version::Entity::find()
            .filter(legal_agreement_version::Column::RealmId.eq(realm_id))
            .filter(legal_agreement_version::Column::AgreementType.eq(agreement_type.as_str()))
            .count(&self.db)
            .await?;
        Ok(count > 0)
    }

    /// Publish a per-realm custom version.
    ///
    /// `version_no = max(version_no of realm's custom rows) + 1`. The
    /// `(COALESCE(realm_id,''), agreement_type, version_no)` expression unique
    /// index guards concurrent publishes: on a unique violation we recompute the
    /// next version_no once and retry; a second violation surfaces as
    /// `LegalError::StaleVersion` → `CoreError::Conflict` so the caller can
    /// re-read and decide. `id` / `published_at` rely on the DB column defaults
    /// (`uuidv7()` / `now()`); `source` is explicitly `custom`.
    async fn publish_custom_version(
        &self,
        realm_id: &str,
        agreement_type: AgreementType,
        content: serde_json::Value,
        label: Option<String>,
        published_by: &str,
    ) -> Result<LegalAgreementVersion, CoreError> {
        // Pre-extract owned/copied captures so the insert closure borrows no
        // value that the retry path must also move. `as_str()` is `&'static str`
        // (Copy); the owned strings are cloned once up front.
        let type_str = agreement_type.as_str();
        let realm_owned = realm_id.to_string();
        let by_owned = published_by.to_string();
        let db = &self.db;

        let attempt = |vno: i32| {
            let active = legal_agreement_version::ActiveModel {
                id: NotSet,
                realm_id: Set(Some(realm_owned.clone())),
                agreement_type: Set(type_str.to_string()),
                version_no: Set(vno),
                version_label: Set(label.clone()),
                content: Set(content.clone()),
                source: Set("custom".to_string()),
                published_at: NotSet,
                published_by: Set(Some(by_owned.clone())),
            };
            active.insert(db)
        };

        let version_no = self
            .next_custom_version_no(realm_id, &agreement_type)
            .await?;

        match attempt(version_no).await {
            Ok(model) => Self::to_domain(model),
            Err(err) if is_scope_type_version_unique_violation(&err) => {
                // Recompute under the now-corrected max and retry once.
                let next = self
                    .next_custom_version_no(realm_id, &agreement_type)
                    .await?;
                match attempt(next).await {
                    Ok(model) => Self::to_domain(model),
                    // Still colliding — a concurrent publish raced ahead twice;
                    // surface a conflict so the caller re-reads current effective.
                    Err(err2) if is_scope_type_version_unique_violation(&err2) => {
                        Err(LegalError::StaleVersion.into())
                    }
                    Err(other) => Err(CoreError::from(other)),
                }
            }
            Err(other) => Err(CoreError::from(other)),
        }
    }
}

/// PostgreSQL implementation of [`UserConsentRepository`].
pub struct PostgresUserConsentRepository {
    db: DatabaseConnection,
}

impl PostgresUserConsentRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Map a `user_agreement_consent` row to the domain entity.
    fn to_domain(model: user_agreement_consent::Model) -> Result<UserAgreementConsent, CoreError> {
        Ok(UserAgreementConsent {
            id: model.id,
            user_id: model.user_id,
            realm_id: model.realm_id,
            agreement_type: AgreementType::try_from(model.agreement_type.as_str())
                .map_err(CoreError::InternalServerError)?,
            consented_version_id: model.consented_version_id,
            consented_at: chrono::DateTime::<chrono::Utc>::from(model.consented_at),
        })
    }
}

impl UserConsentRepository for PostgresUserConsentRepository {
    /// Idempotent upsert on `(user_id, agreement_type)`. An existing row is
    /// updated in place (refresh `consented_version_id`, `consented_at`,
    /// `realm_id`); a missing row is inserted. The unique index
    /// `user_agreement_consent_user_type_unique` is the final arbiter under
    /// concurrent consent for the same user — a collision during the
    /// find-then-insert window is retried as an update.
    async fn upsert_consent(
        &self,
        user_id: Uuid,
        realm_id: &str,
        agreement_type: AgreementType,
        version_id: Uuid,
    ) -> Result<(), CoreError> {
        let existing = user_agreement_consent::Entity::find()
            .filter(user_agreement_consent::Column::UserId.eq(user_id))
            .filter(user_agreement_consent::Column::AgreementType.eq(agreement_type.as_str()))
            .one(&self.db)
            .await?;

        if let Some(model) = existing {
            let mut active: user_agreement_consent::ActiveModel = model.into_active_model();
            active.realm_id = Set(realm_id.to_string());
            active.consented_version_id = Set(version_id);
            // Re-consent is observable: refresh the timestamp explicitly rather
            // than relying on the DB `now()` default (SeaORM omits NotSet from
            // UPDATE, so the old value would otherwise persist).
            active.consented_at = Set(chrono::Utc::now().into());
            active.update(&self.db).await?;
            return Ok(());
        }

        let active = user_agreement_consent::ActiveModel {
            id: NotSet,
            user_id: Set(user_id),
            realm_id: Set(realm_id.to_string()),
            agreement_type: Set(agreement_type.as_str().to_string()),
            consented_version_id: Set(version_id),
            consented_at: NotSet,
        };
        match active.insert(&self.db).await {
            Ok(_) => Ok(()),
            Err(err) if is_user_type_unique_violation(&err) => {
                // Lost the find-then-insert race against another concurrent
                // consent for the same user/type: retry as an update.
                let existing = user_agreement_consent::Entity::find()
                    .filter(user_agreement_consent::Column::UserId.eq(user_id))
                    .filter(
                        user_agreement_consent::Column::AgreementType.eq(agreement_type.as_str()),
                    )
                    .one(&self.db)
                    .await?
                    .ok_or_else(|| {
                        CoreError::DatabaseError(
                            "consent row vanished between insert conflict and update retry"
                                .to_string(),
                        )
                    })?;
                let mut active: user_agreement_consent::ActiveModel = existing.into_active_model();
                active.realm_id = Set(realm_id.to_string());
                active.consented_version_id = Set(version_id);
                active.consented_at = Set(chrono::Utc::now().into());
                active.update(&self.db).await?;
                Ok(())
            }
            Err(other) => Err(CoreError::from(other)),
        }
    }

    async fn get_consent(
        &self,
        user_id: Uuid,
        agreement_type: AgreementType,
    ) -> Result<Option<UserAgreementConsent>, CoreError> {
        let row = user_agreement_consent::Entity::find()
            .filter(user_agreement_consent::Column::UserId.eq(user_id))
            .filter(user_agreement_consent::Column::AgreementType.eq(agreement_type.as_str()))
            .one(&self.db)
            .await?;
        row.map(Self::to_domain).transpose()
    }
}

/// Detect a `legal_agreement_version_scope_type_version_unique` violation.
///
/// sea-orm 1.1 surfaces a Postgres `23505` unique violation as a `Query`/`Exec`
/// wrapping `RuntimeErr::SqlxError`; `PgDatabaseError::code()` returns the
/// SQLSTATE. We additionally match the explicit migration constraint name and a
/// generic `duplicate key` token as message-level fallbacks, mirroring the
/// billing repo's `classify_from_message` resilience against driver/constraint-
/// name drift.
fn is_scope_type_version_unique_violation(err: &DbErr) -> bool {
    if let Some(sqlx_err) = sqlx_error(err) {
        // `PgDatabaseError::code()` is the inherent `&str` SQLSTATE.
        if sqlx_err.code() == "23505" {
            return true;
        }
    }
    let msg = err.to_string();
    msg.contains("legal_agreement_version_scope_type_version_unique")
        || msg.contains("duplicate key value")
}

/// Detect a `user_agreement_consent_user_type_unique` violation (same approach
/// as the agreement-version check, specialized to that index name).
fn is_user_type_unique_violation(err: &DbErr) -> bool {
    if let Some(sqlx_err) = sqlx_error(err)
        && sqlx_err.code() == "23505"
    {
        return true;
    }
    let msg = err.to_string();
    msg.contains("user_agreement_consent_user_type_unique") || msg.contains("duplicate key value")
}

/// Unwrap the underlying sqlx `PgDatabaseError` from a sea-orm `DbErr`, if any.
fn sqlx_error(err: &DbErr) -> Option<&sqlx::postgres::PgDatabaseError> {
    let runtime = match err {
        DbErr::Query(r) | DbErr::Exec(r) | DbErr::Conn(r) => r,
        _ => return None,
    };
    match runtime {
        RuntimeErr::SqlxError(sqlx::error::Error::Database(db)) => db.try_downcast_ref(),
        _ => None,
    }
}
