// PostgreSQL implementation for Points Package repository

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use std::sync::Arc;

use herald_domain::common::entities::app_errors::CoreError;
use herald_domain::points_package::{
    CreatePaymentProviderMappingInput, CreatePointsPackageInput, PackageType, PointsPackage,
    PointsPackagePaymentProvider, PointsPackageRepository,
};
use herald_entity::{
    points_package as points_package_entity, points_package_payment_provider,
    points_package_purchase,
};

/// PostgreSQL implementation of PointsPackage repository
pub struct PostgresPointsPackageRepository {
    db: Arc<DatabaseConnection>,
}

impl PostgresPointsPackageRepository {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    fn model_to_points_package(model: points_package_entity::Model) -> PointsPackage {
        PointsPackage {
            id: model.id,
            realm_id: model.realm_id,
            name: model.name,
            title: model.title,
            description: model.description,
            points: model.points,
            price: model.price,
            currency: model.currency,
            sort_order: model.sort_order,
            enabled: model.enabled,
            package_type: model.package_type.parse().unwrap_or(PackageType::Standard),
            original_price: model.original_price,
            promo_start_time: model.promo_start_time.map(chrono::DateTime::from),
            promo_end_time: model.promo_end_time.map(chrono::DateTime::from),
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        }
    }

    fn model_to_payment_provider(
        model: points_package_payment_provider::Model,
    ) -> PointsPackagePaymentProvider {
        PointsPackagePaymentProvider {
            id: model.id,
            points_package_id: model.points_package_id,
            payment_provider: model.payment_provider,
            enabled: model.enabled,
            external_product_id: model.external_product_id,
            created_at: chrono::DateTime::from(model.created_at),
            updated_at: chrono::DateTime::from(model.updated_at),
        }
    }
}

impl PointsPackageRepository for PostgresPointsPackageRepository {
    async fn create_points_package(
        &self,
        input: CreatePointsPackageInput,
    ) -> Result<PointsPackage, CoreError> {
        let now = chrono::Utc::now();

        let package_model = points_package_entity::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            realm_id: Set(input.realm_id),
            name: Set(input.name),
            title: Set(input.title),
            description: Set(input.description),
            points: Set(input.points),
            price: Set(input.price),
            currency: Set(input.currency),
            sort_order: Set(input.sort_order.unwrap_or(0)),
            enabled: Set(input.enabled.unwrap_or(true)),
            package_type: Set(input
                .package_type
                .map(|pt| pt.to_string())
                .unwrap_or_else(|| "standard".to_string())),
            original_price: Set(input.original_price),
            promo_start_time: Set(input.promo_start_time.map(|t| t.into())),
            promo_end_time: Set(input.promo_end_time.map(|t| t.into())),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let result = package_model.insert(self.db.as_ref()).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to create points package: {e}"))
        })?;

        Ok(Self::model_to_points_package(result))
    }

    async fn find_points_package_by_id(
        &self,
        realm_id: &str,
        package_id: uuid::Uuid,
    ) -> Result<Option<PointsPackage>, CoreError> {
        let result = points_package_entity::Entity::find_by_id(package_id)
            .filter(points_package_entity::Column::RealmId.eq(realm_id))
            .one(self.db.as_ref())
            .await
            .map_err(|e| CoreError::DatabaseError(format!("Failed to find points package: {e}")))?;

        Ok(result.map(Self::model_to_points_package))
    }

    async fn find_points_package_by_name(
        &self,
        realm_id: &str,
        name: &str,
    ) -> Result<Option<PointsPackage>, CoreError> {
        let result = points_package_entity::Entity::find()
            .filter(points_package_entity::Column::RealmId.eq(realm_id))
            .filter(points_package_entity::Column::Name.eq(name))
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to find points package by name: {e}"))
            })?;

        Ok(result.map(Self::model_to_points_package))
    }

    async fn list_points_packages(
        &self,
        realm_id: &str,
        enabled_only: bool,
    ) -> Result<Vec<PointsPackage>, CoreError> {
        let mut query = points_package_entity::Entity::find()
            .filter(points_package_entity::Column::RealmId.eq(realm_id));

        if enabled_only {
            query = query.filter(points_package_entity::Column::Enabled.eq(true));
        }

        let results = query
            .order_by_desc(points_package_entity::Column::SortOrder)
            .order_by_asc(points_package_entity::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to list points packages: {e}"))
            })?;

        Ok(results
            .into_iter()
            .map(Self::model_to_points_package)
            .collect())
    }

    async fn update_points_package(
        &self,
        package: PointsPackage,
    ) -> Result<PointsPackage, CoreError> {
        let package_model = points_package_entity::ActiveModel {
            id: Set(package.id),
            realm_id: Set(package.realm_id),
            name: Set(package.name),
            title: Set(package.title),
            description: Set(package.description),
            points: Set(package.points),
            price: Set(package.price),
            currency: Set(package.currency),
            sort_order: Set(package.sort_order),
            enabled: Set(package.enabled),
            package_type: Set(package.package_type.to_string()),
            original_price: Set(package.original_price),
            promo_start_time: Set(package.promo_start_time.map(|t| t.into())),
            promo_end_time: Set(package.promo_end_time.map(|t| t.into())),
            created_at: Set(package.created_at.into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = package_model.update(self.db.as_ref()).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to update points package: {e}"))
        })?;

        Ok(Self::model_to_points_package(result))
    }

    async fn delete_points_package(
        &self,
        realm_id: &str,
        package_id: uuid::Uuid,
    ) -> Result<(), CoreError> {
        points_package_entity::Entity::delete_by_id(package_id)
            .filter(points_package_entity::Column::RealmId.eq(realm_id))
            .exec(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to delete points package: {e}"))
            })?;

        Ok(())
    }

    async fn create_payment_provider_mapping(
        &self,
        input: CreatePaymentProviderMappingInput,
    ) -> Result<PointsPackagePaymentProvider, CoreError> {
        let now = chrono::Utc::now();

        let mapping_model = points_package_payment_provider::ActiveModel {
            id: Set(uuid::Uuid::now_v7()),
            points_package_id: Set(input.points_package_id),
            payment_provider: Set(input.payment_provider),
            enabled: Set(input.enabled),
            external_product_id: Set(input.external_product_id),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let result = mapping_model.insert(self.db.as_ref()).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to create payment provider mapping: {e}"))
        })?;

        Ok(Self::model_to_payment_provider(result))
    }

    async fn list_payment_provider_mappings(
        &self,
        package_id: uuid::Uuid,
    ) -> Result<Vec<PointsPackagePaymentProvider>, CoreError> {
        let results = points_package_payment_provider::Entity::find()
            .filter(points_package_payment_provider::Column::PointsPackageId.eq(package_id))
            .all(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to list payment provider mappings: {e}"))
            })?;

        Ok(results
            .into_iter()
            .map(Self::model_to_payment_provider)
            .collect())
    }

    async fn find_payment_provider_mapping_by_id(
        &self,
        mapping_id: uuid::Uuid,
    ) -> Result<Option<PointsPackagePaymentProvider>, CoreError> {
        let result = points_package_payment_provider::Entity::find_by_id(mapping_id)
            .one(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to find payment provider mapping: {e}"))
            })?;

        Ok(result.map(Self::model_to_payment_provider))
    }

    async fn update_payment_provider_mapping(
        &self,
        mapping: PointsPackagePaymentProvider,
    ) -> Result<PointsPackagePaymentProvider, CoreError> {
        let mapping_model = points_package_payment_provider::ActiveModel {
            id: Set(mapping.id),
            points_package_id: Set(mapping.points_package_id),
            payment_provider: Set(mapping.payment_provider),
            enabled: Set(mapping.enabled),
            external_product_id: Set(mapping.external_product_id),
            created_at: Set(mapping.created_at.into()),
            updated_at: Set(chrono::Utc::now().into()),
        };

        let result = mapping_model.update(self.db.as_ref()).await.map_err(|e| {
            CoreError::DatabaseError(format!("Failed to update payment provider mapping: {e}"))
        })?;

        Ok(Self::model_to_payment_provider(result))
    }

    async fn delete_payment_provider_mapping(
        &self,
        mapping_id: uuid::Uuid,
    ) -> Result<(), CoreError> {
        points_package_payment_provider::Entity::delete_by_id(mapping_id)
            .exec(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!(
                    "Failed to delete payment provider mapping: {}",
                    e
                ))
            })?;

        Ok(())
    }

    async fn has_purchase_records(&self, package_id: uuid::Uuid) -> Result<bool, CoreError> {
        let count = points_package_purchase::Entity::find()
            .filter(points_package_purchase::Column::PointsPackageId.eq(package_id))
            .count(self.db.as_ref())
            .await
            .map_err(|e| {
                CoreError::DatabaseError(format!("Failed to check purchase records: {e}"))
            })?;

        Ok(count > 0)
    }
}
