use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use std::sync::Arc;
use uuid::Uuid;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::user::{
    entities::{Profile, User, UserStatus},
    ports::{UserRepository, UserVerificationRepository},
    value_objects::{CreateUserRequest, UpdateUserRequest},
};
use herald_entity::{account, profile};

pub struct PostgresUserRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresUserRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn to_domain_user(model: &account::Model, nickname: Option<String>) -> User {
        // Validate the ID is a proper UUID
        let id_str = model.id.to_string();
        if id_str.len() != 36 {
            tracing::error!(
                id = %id_str,
                length = id_str.len(),
                "Invalid UUID length in to_domain_user conversion"
            );
        }

        // Validate the UUID format
        if uuid::Uuid::parse_str(&id_str).is_err() {
            tracing::error!(
                id = %id_str,
                "Invalid UUID format in to_domain_user conversion"
            );
        }

        User {
            id: model.id,
            realm_id: model.realm_id.clone().unwrap_or_default(),
            email: model.email.clone(),
            nickname,
            password_hash: model.password.clone(),
            provider_ids: model.provider_ids.clone(),
            status: UserStatus::from(model.status),
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
        }
    }

    fn to_domain_profile(model: &profile::Model) -> Profile {
        Profile {
            id: model.id,
            realm_id: model.realm_id.clone().unwrap_or_default(),
            nickname: model.nickname.clone(),
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
        }
    }
}

impl UserRepository for PostgresUserRepository {
    async fn create_user(
        &self,
        request: CreateUserRequest,
        password_hash: String,
    ) -> Result<User, CoreError> {
        let now = chrono::Utc::now();
        // 使用 UUID v7 生成用户 ID
        let user_id = herald_domain::common::entities::generate_uuid_v7();

        let active_model = account::ActiveModel {
            id: sea_orm::Set(user_id),
            realm_id: sea_orm::Set(Some(request.realm_id)),
            email: sea_orm::Set(request.email),
            username: sea_orm::Set(None), // Explicitly set username to None
            password: sea_orm::Set(Some(password_hash)),
            provider_ids: sea_orm::Set(request.provider_ids.unwrap_or_default()),
            status: sea_orm::Set(UserStatus::WaitVerified.into()),
            created_at: sea_orm::Set(now.into()),
            updated_at: sea_orm::Set(now.into()),
        };

        let result = active_model.insert(&*self.db).await?;
        Ok(Self::to_domain_user(&result, None))
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<User, CoreError> {
        tracing::debug!("Querying user by ID: {}", id);

        let result = account::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?;

        // Verify the ID matches
        tracing::debug!("Found user: id={}, email={}", result.id, result.email);

        if result.id != id {
            tracing::error!(
                query_id = %id,
                result_id = %result.id,
                "User ID mismatch in database query result"
            );
            return Err(CoreError::NotFound);
        }

        Ok(Self::to_domain_user(&result, None))
    }

    async fn get_user_by_email(&self, realm_id: &str, email: &str) -> Result<User, CoreError> {
        let result = account::Entity::find()
            .filter(account::Column::RealmId.eq(realm_id))
            .filter(account::Column::Email.eq(email))
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?;

        Ok(Self::to_domain_user(&result, None))
    }

    async fn get_user_by_email_or_username(
        &self,
        realm_id: &str,
        email: Option<String>,
        username: Option<String>,
    ) -> Result<Option<(Uuid, Option<String>, i16)>, CoreError> {
        let mut query = account::Entity::find().filter(account::Column::RealmId.eq(realm_id));

        // Add email or username filter
        if let Some(email) = email {
            query = query.filter(account::Column::Email.eq(email));
        } else if let Some(username) = username {
            query = query.filter(account::Column::Username.eq(username));
        } else {
            // Neither email nor username provided
            return Ok(None);
        }

        let result = query.one(&*self.db).await?;

        Ok(result.map(|model| (model.id, model.password, model.status)))
    }

    async fn change_password(
        &self,
        realm_id: &str,
        user_id: Uuid,
        new_password_hash: String,
    ) -> Result<(), CoreError> {
        let mut active_model: account::ActiveModel = account::Entity::find()
            .filter(account::Column::RealmId.eq(realm_id))
            .filter(account::Column::Id.eq(user_id))
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?
            .into();

        active_model.password = sea_orm::Set(Some(new_password_hash));
        active_model.updated_at = sea_orm::Set(chrono::Utc::now().into());

        active_model.update(&*self.db).await?;
        Ok(())
    }

    async fn update_user_status(&self, user_id: Uuid, status: i16) -> Result<(), CoreError> {
        let mut active_model: account::ActiveModel = account::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?
            .into();

        active_model.status = sea_orm::Set(status);
        active_model.updated_at = sea_orm::Set(chrono::Utc::now().into());

        active_model.update(&*self.db).await?;
        Ok(())
    }

    async fn list_users(
        &self,
        realm_id: &str,
        page: u64,
        page_size: u64,
        email: Option<String>,
    ) -> Result<(Vec<User>, i64), CoreError> {
        let page = page.max(1);
        let page_size = page_size.min(100);
        let offset = (page - 1) * page_size;

        let mut query = account::Entity::find().filter(account::Column::RealmId.eq(realm_id));

        // Add email filter if provided
        if let Some(email_filter) = email {
            query = query.filter(account::Column::Email.contains(email_filter));
        }

        let total = query.clone().count(&*self.db).await?;

        let results = query
            .order_by_desc(account::Column::CreatedAt)
            .limit(page_size)
            .offset(offset)
            .all(&*self.db)
            .await?;

        let users = results
            .iter()
            .map(|model| Self::to_domain_user(model, None))
            .collect();
        Ok((users, total as i64))
    }

    async fn update_user(&self, id: Uuid, request: UpdateUserRequest) -> Result<User, CoreError> {
        let mut active_model: account::ActiveModel = account::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?
            .into();

        if let Some(status) = request.status {
            active_model.status = sea_orm::Set(status);
        }

        active_model.updated_at = sea_orm::Set(chrono::Utc::now().into());

        let result = active_model.update(&*self.db).await?;
        Ok(Self::to_domain_user(&result, None))
    }

    async fn delete_user(&self, id: Uuid) -> Result<(), CoreError> {
        account::Entity::delete_by_id(id).exec(&*self.db).await?;

        Ok(())
    }

    async fn create_profile(&self, profile: Profile) -> Result<Profile, CoreError> {
        let active_model = profile::ActiveModel {
            id: sea_orm::Set(profile.id),
            realm_id: sea_orm::Set(Some(profile.realm_id.clone())),
            nickname: sea_orm::Set(profile.nickname.clone()),
            created_at: sea_orm::Set(profile.created_at.into()),
            updated_at: sea_orm::Set(profile.updated_at.into()),
        };

        active_model.insert(&*self.db).await?;
        Ok(profile)
    }

    async fn get_profile(&self, user_id: Uuid) -> Result<Profile, CoreError> {
        let result = profile::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?;

        Ok(Self::to_domain_profile(&result))
    }

    async fn update_profile(
        &self,
        user_id: Uuid,
        nickname: Option<String>,
    ) -> Result<Profile, CoreError> {
        let mut active_model: profile::ActiveModel = profile::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or(CoreError::NotFound)?
            .into();

        if let Some(nickname) = nickname {
            active_model.nickname = sea_orm::Set(Some(nickname));
        }

        active_model.updated_at = sea_orm::Set(chrono::Utc::now().into());

        let result = active_model.update(&*self.db).await?;
        Ok(Self::to_domain_profile(&result))
    }
}

pub struct PostgresVerificationRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresVerificationRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

impl UserVerificationRepository for PostgresVerificationRepository {
    async fn create_verification_code(
        &self,
        email: &str,
        code_type: &str,
        code: &str,
    ) -> Result<(), CoreError> {
        use herald_entity::email_verification_code;

        let now = chrono::Utc::now();
        // 使用 UUID v7 生成验证码 ID
        let id = herald_domain::common::entities::generate_uuid_v7();

        let active_model = email_verification_code::ActiveModel {
            id: sea_orm::Set(id),
            email: sea_orm::Set(email.to_string()),
            r#type: sea_orm::Set(code_type.to_string()),
            verification_code: sea_orm::Set(code.to_string()),
            created_at: sea_orm::Set(now.into()),
        };

        active_model.insert(&*self.db).await?;
        Ok(())
    }

    async fn verify_code(
        &self,
        email: &str,
        code_type: &str,
        code: &str,
    ) -> Result<bool, CoreError> {
        use herald_entity::email_verification_code;

        let result = email_verification_code::Entity::find()
            .filter(email_verification_code::Column::Email.eq(email))
            .filter(email_verification_code::Column::Type.eq(code_type))
            .filter(email_verification_code::Column::VerificationCode.eq(code))
            .one(&*self.db)
            .await?;

        Ok(result.is_some())
    }

    async fn consume_code(&self, code: &str) -> Result<(), CoreError> {
        use herald_entity::email_verification_code;

        email_verification_code::Entity::delete_many()
            .filter(email_verification_code::Column::VerificationCode.eq(code))
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    async fn get_email_by_code(&self, code: &str) -> Result<Option<String>, CoreError> {
        use herald_entity::email_verification_code;

        let result = email_verification_code::Entity::find()
            .filter(email_verification_code::Column::VerificationCode.eq(code))
            .one(&*self.db)
            .await?;

        Ok(result.map(|r| r.email))
    }

    async fn delete_code_by_type(&self, email: &str, code_type: &str) -> Result<(), CoreError> {
        use herald_entity::email_verification_code;

        email_verification_code::Entity::delete_many()
            .filter(email_verification_code::Column::Email.eq(email))
            .filter(email_verification_code::Column::Type.eq(code_type))
            .exec(&*self.db)
            .await?;

        Ok(())
    }
}
