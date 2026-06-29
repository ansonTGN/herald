use std::sync::Arc;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::realm::RealmPointsConfigInitializer;
use herald_entity::realm_default_config;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

pub struct PostgresRealmPointsConfigInitializer {
    db: Arc<DatabaseConnection>,
}

impl PostgresRealmPointsConfigInitializer {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

impl RealmPointsConfigInitializer for PostgresRealmPointsConfigInitializer {
    fn create_default_realm_points_config(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<(), CoreError>> + Send {
        let db = self.db.clone();
        let realm_id = realm_id.to_string();

        async move {
            let now = chrono::Utc::now();
            let active_model = realm_default_config::ActiveModel {
                realm_id: Set(realm_id),
                registration_bonus_points: Set(0),
                free_periodic_points_amount: Set(0),
                free_periodic_grant_period_type: Set("once".to_string()),
                free_periodic_validity_days: Set(0),
                // free_periodic_quota_windows: NULL ⟹ no window-model grant
                // (BE-D07 field wiring; default Realm has no free periodic quota).
                free_periodic_quota_windows: Set(None),
                created_at: Set(now.into()),
                updated_at: Set(now.into()),
            };

            active_model
                .insert(&*db)
                .await
                .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

            Ok(())
        }
    }
}
