use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::sync::Arc;
use uuid::Uuid;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::user_passkey::entities::UserPasskeyCredential;
use herald_domain::user_passkey::ports::UserPasskeyRepository;
use herald_entity::user_passkey_credential;

pub struct PostgresUserPasskeyRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresUserPasskeyRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn to_domain(
        model: user_passkey_credential::Model,
    ) -> Result<UserPasskeyCredential, CoreError> {
        let counter = u64::try_from(model.counter).map_err(|_| {
            CoreError::DatabaseError("passkey counter is negative in storage".to_string())
        })?;
        let transports = serde_json::from_value::<Vec<String>>(model.transports)?;

        Ok(UserPasskeyCredential {
            id: model.id,
            user_id: model.user_id,
            realm_id: model.realm_id,
            credential_id: model.credential_id,
            credential_public_key: model.credential_public_key,
            counter,
            transports,
            aaguid: model.aaguid,
            backup_eligible: model.backup_eligible,
            backup_state: model.backup_state,
            user_verified: model.user_verified,
            nickname: model.nickname,
            last_used_at: model.last_used_at.map(|dt| dt.into()),
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
        })
    }

    fn counter_to_i64(counter: u64) -> Result<i64, CoreError> {
        i64::try_from(counter).map_err(|_| {
            CoreError::DatabaseError("passkey counter exceeds BIGINT range".to_string())
        })
    }
}

impl UserPasskeyRepository for PostgresUserPasskeyRepository {
    async fn list_by_user(
        &self,
        realm_id: &str,
        user_id: Uuid,
    ) -> Result<Vec<UserPasskeyCredential>, CoreError> {
        let results = user_passkey_credential::Entity::find()
            .filter(user_passkey_credential::Column::RealmId.eq(realm_id))
            .filter(user_passkey_credential::Column::UserId.eq(user_id))
            .all(&*self.db)
            .await?;

        results.into_iter().map(Self::to_domain).collect()
    }

    async fn find_by_credential_id(
        &self,
        realm_id: &str,
        credential_id: &[u8],
    ) -> Result<Option<UserPasskeyCredential>, CoreError> {
        let result = user_passkey_credential::Entity::find()
            .filter(user_passkey_credential::Column::RealmId.eq(realm_id))
            .filter(user_passkey_credential::Column::CredentialId.eq(credential_id.to_vec()))
            .one(&*self.db)
            .await?;

        result.map(Self::to_domain).transpose()
    }

    async fn insert(
        &self,
        credential: UserPasskeyCredential,
    ) -> Result<UserPasskeyCredential, CoreError> {
        let active_model = user_passkey_credential::ActiveModel {
            id: Set(credential.id),
            user_id: Set(credential.user_id),
            realm_id: Set(credential.realm_id),
            credential_id: Set(credential.credential_id),
            credential_public_key: Set(credential.credential_public_key),
            counter: Set(Self::counter_to_i64(credential.counter)?),
            transports: Set(serde_json::to_value(credential.transports)?),
            aaguid: Set(credential.aaguid),
            backup_eligible: Set(credential.backup_eligible),
            backup_state: Set(credential.backup_state),
            user_verified: Set(credential.user_verified),
            nickname: Set(credential.nickname),
            last_used_at: Set(credential.last_used_at.map(|dt| dt.into())),
            created_at: Set(credential.created_at.into()),
            updated_at: Set(credential.updated_at.into()),
        };

        let result = active_model.insert(&*self.db).await?;
        Self::to_domain(result)
    }

    async fn rename(
        &self,
        realm_id: &str,
        user_id: Uuid,
        id: Uuid,
        nickname: &str,
    ) -> Result<(), CoreError> {
        let credential = user_passkey_credential::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?;

        if credential.realm_id != realm_id || credential.user_id != user_id {
            return Err(CoreError::Forbidden(
                "passkey credential does not belong to user in realm".to_string(),
            ));
        }

        let mut active_model: user_passkey_credential::ActiveModel = credential.into();
        active_model.nickname = Set(Some(nickname.to_string()));
        active_model.updated_at = Set(chrono::Utc::now().into());
        active_model.update(&*self.db).await?;

        Ok(())
    }

    async fn delete(&self, realm_id: &str, user_id: Uuid, id: Uuid) -> Result<(), CoreError> {
        let credential = user_passkey_credential::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?;

        if credential.realm_id != realm_id || credential.user_id != user_id {
            return Err(CoreError::Forbidden(
                "passkey credential does not belong to user in realm".to_string(),
            ));
        }

        user_passkey_credential::Entity::delete_by_id(id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    async fn update_counter_and_used(
        &self,
        id: Uuid,
        counter: u64,
        user_verified: bool,
        used_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), CoreError> {
        let credential = user_passkey_credential::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?;

        let mut active_model: user_passkey_credential::ActiveModel = credential.into();
        active_model.counter = Set(Self::counter_to_i64(counter)?);
        active_model.user_verified = Set(user_verified);
        active_model.last_used_at = Set(Some(used_at.into()));
        active_model.updated_at = Set(chrono::Utc::now().into());
        active_model.update(&*self.db).await?;

        Ok(())
    }
}
