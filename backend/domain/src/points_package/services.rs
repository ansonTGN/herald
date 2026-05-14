// Points Package domain service

use std::sync::Arc;

use super::entities::{PointsPackage, PointsPackagePaymentProvider};
use super::errors::{PointsPackageErrorExt, PointsPackageResult};
use super::ports::{
    CreatePaymentProviderMappingInput, CreatePointsPackageInput, PointsPackageRepository,
    UpdatePaymentProviderMappingInput, UpdatePointsPackageInput,
};
use crate::authentication::Identity;
use crate::authorization::PermissionService;
use crate::common::entities::app_errors::CoreError;

/// Points Package service
pub struct PointsPackageService<R: PointsPackageRepository> {
    repository: Arc<R>,
}

impl<R: PointsPackageRepository> PointsPackageService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    async fn ensure_realm_access(
        &self,
        identity: &Identity,
        realm_id: &str,
    ) -> Result<(), CoreError> {
        if !identity.has_access_to_realm(realm_id) {
            return Err(CoreError::Forbidden(
                "Access denied: cross-realm points package access is not allowed".to_string(),
            ));
        }

        Ok(())
    }

    async fn ensure_realm_admin<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
    ) -> Result<(), CoreError> {
        self.ensure_realm_access(identity, realm_id).await?;

        if !identity.is_user() {
            return Err(CoreError::Forbidden(
                "Access denied: realm admin user required".to_string(),
            ));
        }

        let allowed = permission_checker
            .check_permission(realm_id, &identity.user_id(), "realm", "admin")
            .await
            .map_err(|e| CoreError::InternalServerError(format!("Permission check failed: {e}")))?;

        if !allowed {
            return Err(CoreError::Forbidden(
                "Access denied: realm admin permission required".to_string(),
            ));
        }

        Ok(())
    }

    pub async fn create_points_package_authorized<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        input: CreatePointsPackageInput,
    ) -> PointsPackageResult<PointsPackage> {
        self.ensure_realm_admin(identity, permission_checker, realm_id)
            .await?;
        self.create_points_package(realm_id, input).await
    }

    pub async fn list_visible_points_packages(
        &self,
        identity: &Identity,
        realm_id: &str,
    ) -> PointsPackageResult<Vec<PointsPackage>> {
        self.ensure_realm_access(identity, realm_id).await?;
        self.list_points_packages(realm_id, true).await
    }

    pub async fn get_visible_points_package(
        &self,
        identity: &Identity,
        realm_id: &str,
        package_id: uuid::Uuid,
    ) -> PointsPackageResult<PointsPackage> {
        self.ensure_realm_access(identity, realm_id).await?;

        let package = self.get_points_package(realm_id, package_id).await?;
        if !package.enabled {
            return Err(CoreError::package_not_found(&package_id.to_string()));
        }

        Ok(package)
    }

    pub async fn update_points_package_authorized<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        package_id: uuid::Uuid,
        input: UpdatePointsPackageInput,
    ) -> PointsPackageResult<PointsPackage> {
        self.ensure_realm_admin(identity, permission_checker, realm_id)
            .await?;
        self.update_points_package(realm_id, package_id, input)
            .await
    }

    pub async fn delete_points_package_authorized<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        package_id: uuid::Uuid,
    ) -> PointsPackageResult<()> {
        self.ensure_realm_admin(identity, permission_checker, realm_id)
            .await?;
        self.delete_points_package(realm_id, package_id).await
    }

    pub async fn list_payment_provider_mappings_authorized<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        package_id: uuid::Uuid,
    ) -> PointsPackageResult<Vec<PointsPackagePaymentProvider>> {
        self.ensure_realm_admin(identity, permission_checker, realm_id)
            .await?;
        self.get_points_package(realm_id, package_id).await?;
        self.list_payment_provider_mappings(package_id).await
    }

    pub async fn add_payment_provider_mapping_authorized<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        input: CreatePaymentProviderMappingInput,
    ) -> PointsPackageResult<PointsPackagePaymentProvider> {
        self.ensure_realm_admin(identity, permission_checker, realm_id)
            .await?;
        self.get_points_package(realm_id, input.points_package_id)
            .await?;
        self.add_payment_provider_mapping(input).await
    }

    pub async fn update_payment_provider_mapping_authorized<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        package_id: uuid::Uuid,
        mapping_id: uuid::Uuid,
        input: UpdatePaymentProviderMappingInput,
    ) -> PointsPackageResult<PointsPackagePaymentProvider> {
        self.ensure_realm_admin(identity, permission_checker, realm_id)
            .await?;
        self.get_points_package(realm_id, package_id).await?;

        let mapping = self
            .find_payment_provider_mapping(mapping_id)
            .await?
            .ok_or_else(|| CoreError::package_not_found(&mapping_id.to_string()))?;

        if mapping.points_package_id != package_id {
            return Err(CoreError::package_not_found(&mapping_id.to_string()));
        }

        self.update_payment_provider_mapping(mapping_id, input)
            .await
    }

    pub async fn remove_payment_provider_mapping_authorized<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        package_id: uuid::Uuid,
        mapping_id: uuid::Uuid,
    ) -> PointsPackageResult<()> {
        self.ensure_realm_admin(identity, permission_checker, realm_id)
            .await?;
        self.get_points_package(realm_id, package_id).await?;

        let mapping = self
            .find_payment_provider_mapping(mapping_id)
            .await?
            .ok_or_else(|| CoreError::package_not_found(&mapping_id.to_string()))?;

        if mapping.points_package_id != package_id {
            return Err(CoreError::package_not_found(&mapping_id.to_string()));
        }

        self.remove_payment_provider_mapping(mapping_id).await
    }

    /// Create a new points package
    pub async fn create_points_package(
        &self,
        realm_id: &str,
        input: CreatePointsPackageInput,
    ) -> PointsPackageResult<PointsPackage> {
        // Validate points > 0
        if input.points <= 0 {
            return Err(CoreError::invalid_points_amount(input.points));
        }

        // Validate price > 0
        if input.price <= 0 {
            return Err(CoreError::invalid_price(input.price));
        }

        // Check name uniqueness
        if let Some(_existing) = self
            .repository
            .find_points_package_by_name(realm_id, &input.name)
            .await?
        {
            return Err(CoreError::name_already_exists(&input.name));
        }

        self.repository.create_points_package(input).await
    }

    /// Get a points package by ID
    pub async fn get_points_package(
        &self,
        realm_id: &str,
        package_id: uuid::Uuid,
    ) -> PointsPackageResult<PointsPackage> {
        self.repository
            .find_points_package_by_id(realm_id, package_id)
            .await?
            .ok_or_else(|| CoreError::package_not_found(&package_id.to_string()))
    }

    /// List all points packages in a realm
    pub async fn list_points_packages(
        &self,
        realm_id: &str,
        enabled_only: bool,
    ) -> PointsPackageResult<Vec<PointsPackage>> {
        self.repository
            .list_points_packages(realm_id, enabled_only)
            .await
    }

    /// Update a points package
    pub async fn update_points_package(
        &self,
        realm_id: &str,
        package_id: uuid::Uuid,
        input: UpdatePointsPackageInput,
    ) -> PointsPackageResult<PointsPackage> {
        // Fetch existing package
        let mut package = self.get_points_package(realm_id, package_id).await?;

        // Update fields
        if let Some(title) = input.title {
            package.title = title;
        }
        if let Some(description) = input.description {
            package.description = Some(description);
        }
        if let Some(price) = input.price {
            if price <= 0 {
                return Err(CoreError::invalid_price(price));
            }
            package.price = price;
        }
        if let Some(currency) = input.currency {
            package.currency = currency;
        }
        if let Some(sort_order) = input.sort_order {
            package.sort_order = sort_order;
        }
        if let Some(enabled) = input.enabled {
            package.enabled = enabled;
        }

        self.repository.update_points_package(package).await
    }

    /// Delete a points package
    pub async fn delete_points_package(
        &self,
        realm_id: &str,
        package_id: uuid::Uuid,
    ) -> PointsPackageResult<()> {
        // Check for existing purchase records
        if self.repository.has_purchase_records(package_id).await? {
            return Err(CoreError::has_purchase_records());
        }

        self.repository
            .delete_points_package(realm_id, package_id)
            .await
    }

    /// Add a payment provider mapping to a package
    pub async fn add_payment_provider_mapping(
        &self,
        input: CreatePaymentProviderMappingInput,
    ) -> PointsPackageResult<PointsPackagePaymentProvider> {
        // Check uniqueness of payment provider for this package
        let existing = self
            .repository
            .list_payment_provider_mappings(input.points_package_id)
            .await?;

        if existing
            .iter()
            .any(|m| m.payment_provider == input.payment_provider)
        {
            return Err(CoreError::payment_provider_already_configured(
                &input.payment_provider,
            ));
        }

        self.repository.create_payment_provider_mapping(input).await
    }

    /// List payment provider mappings for a package
    pub async fn list_payment_provider_mappings(
        &self,
        package_id: uuid::Uuid,
    ) -> PointsPackageResult<Vec<PointsPackagePaymentProvider>> {
        self.repository
            .list_payment_provider_mappings(package_id)
            .await
    }

    /// Find a payment provider mapping by ID
    pub async fn find_payment_provider_mapping(
        &self,
        mapping_id: uuid::Uuid,
    ) -> PointsPackageResult<Option<PointsPackagePaymentProvider>> {
        self.repository
            .find_payment_provider_mapping_by_id(mapping_id)
            .await
    }

    /// Update a payment provider mapping
    pub async fn update_payment_provider_mapping(
        &self,
        mapping_id: uuid::Uuid,
        input: UpdatePaymentProviderMappingInput,
    ) -> PointsPackageResult<PointsPackagePaymentProvider> {
        // Fetch existing mapping
        let mapping = self
            .repository
            .find_payment_provider_mapping_by_id(mapping_id)
            .await?;
        let mut mapping =
            mapping.ok_or_else(|| CoreError::package_not_found(&mapping_id.to_string()))?;

        // Update fields
        mapping.external_product_id = input.external_product_id;
        mapping.enabled = input.enabled;

        self.repository
            .update_payment_provider_mapping(mapping)
            .await
    }

    /// Remove a payment provider mapping from a package
    pub async fn remove_payment_provider_mapping(
        &self,
        mapping_id: uuid::Uuid,
    ) -> PointsPackageResult<()> {
        self.repository
            .delete_payment_provider_mapping(mapping_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use uuid::Uuid;

    // Mock repository for testing
    struct MockPointsPackageRepository;

    impl PointsPackageRepository for MockPointsPackageRepository {
        async fn create_points_package(
            &self,
            _input: CreatePointsPackageInput,
        ) -> Result<PointsPackage, CoreError> {
            Ok(PointsPackage {
                id: Uuid::now_v7(),
                realm_id: "test-realm".to_string(),
                name: "test-package".to_string(),
                title: "Test Package".to_string(),
                description: None,
                points: 500,
                price: 2999,
                currency: "USD".to_string(),
                sort_order: 0,
                enabled: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        async fn find_points_package_by_id(
            &self,
            _realm_id: &str,
            _package_id: Uuid,
        ) -> Result<Option<PointsPackage>, CoreError> {
            Ok(None)
        }

        async fn find_points_package_by_name(
            &self,
            _realm_id: &str,
            _name: &str,
        ) -> Result<Option<PointsPackage>, CoreError> {
            Ok(None)
        }

        async fn list_points_packages(
            &self,
            _realm_id: &str,
            _enabled_only: bool,
        ) -> Result<Vec<PointsPackage>, CoreError> {
            Ok(vec![])
        }

        async fn update_points_package(
            &self,
            _package: PointsPackage,
        ) -> Result<PointsPackage, CoreError> {
            Ok(PointsPackage {
                id: Uuid::now_v7(),
                realm_id: "test-realm".to_string(),
                name: "test-package".to_string(),
                title: "Updated Package".to_string(),
                description: None,
                points: 500,
                price: 2999,
                currency: "USD".to_string(),
                sort_order: 0,
                enabled: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        async fn delete_points_package(
            &self,
            _realm_id: &str,
            _package_id: Uuid,
        ) -> Result<(), CoreError> {
            Ok(())
        }

        async fn create_payment_provider_mapping(
            &self,
            _input: CreatePaymentProviderMappingInput,
        ) -> Result<PointsPackagePaymentProvider, CoreError> {
            Ok(PointsPackagePaymentProvider {
                id: Uuid::now_v7(),
                points_package_id: Uuid::now_v7(),
                payment_provider: "wechat".to_string(),
                enabled: true,
                external_product_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        async fn list_payment_provider_mappings(
            &self,
            _package_id: Uuid,
        ) -> Result<Vec<PointsPackagePaymentProvider>, CoreError> {
            Ok(vec![])
        }

        async fn find_payment_provider_mapping_by_id(
            &self,
            _mapping_id: Uuid,
        ) -> Result<Option<PointsPackagePaymentProvider>, CoreError> {
            Ok(None)
        }

        async fn update_payment_provider_mapping(
            &self,
            _mapping: PointsPackagePaymentProvider,
        ) -> Result<PointsPackagePaymentProvider, CoreError> {
            Ok(PointsPackagePaymentProvider {
                id: Uuid::now_v7(),
                points_package_id: Uuid::now_v7(),
                payment_provider: "wechat".to_string(),
                enabled: true,
                external_product_id: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
        }

        async fn delete_payment_provider_mapping(
            &self,
            _mapping_id: Uuid,
        ) -> Result<(), CoreError> {
            Ok(())
        }

        async fn has_purchase_records(&self, _package_id: Uuid) -> Result<bool, CoreError> {
            Ok(false)
        }
    }
    #[tokio::test]
    async fn test_create_points_package_invalid_points() {
        let repo = Arc::new(MockPointsPackageRepository);
        let service = PointsPackageService::new(repo);

        let input = CreatePointsPackageInput {
            realm_id: "test-realm".to_string(),
            name: "test-package".to_string(),
            title: "Test Package".to_string(),
            description: None,
            points: -100, // Invalid
            price: 2999,
            currency: "USD".to_string(),
            sort_order: None,
            enabled: None,
        };

        let result = service.create_points_package("test-realm", input).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::InvalidPointsAmount(amount) => {
                assert_eq!(amount, -100);
            }
            _ => panic!("Expected InvalidPointsAmount error"),
        }
    }

    #[tokio::test]
    async fn test_create_points_package_invalid_price() {
        let repo = Arc::new(MockPointsPackageRepository);
        let service = PointsPackageService::new(repo);

        let input = CreatePointsPackageInput {
            realm_id: "test-realm".to_string(),
            name: "test-package".to_string(),
            title: "Test Package".to_string(),
            description: None,
            points: 500,
            price: -100, // Invalid
            currency: "USD".to_string(),
            sort_order: None,
            enabled: None,
        };

        let result = service.create_points_package("test-realm", input).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::InvalidPrice(amount) => {
                assert_eq!(amount, -100);
            }
            _ => panic!("Expected InvalidPrice error"),
        }
    }

    #[tokio::test]
    async fn test_create_points_package_duplicate_name() {
        struct DuplicateNameMockRepository;

        impl PointsPackageRepository for DuplicateNameMockRepository {
            async fn find_points_package_by_name(
                &self,
                _realm_id: &str,
                _name: &str,
            ) -> Result<Option<PointsPackage>, CoreError> {
                // Simulate existing package with same name
                Ok(Some(PointsPackage {
                    id: Uuid::now_v7(),
                    realm_id: "test-realm".to_string(),
                    name: "test-package".to_string(),
                    title: "Existing Package".to_string(),
                    description: None,
                    points: 100,
                    price: 999,
                    currency: "USD".to_string(),
                    sort_order: 0,
                    enabled: true,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            }

            async fn create_points_package(
                &self,
                _input: CreatePointsPackageInput,
            ) -> Result<PointsPackage, CoreError> {
                unreachable!("Should not create when name exists")
            }

            // Default implementations for other methods
            async fn find_points_package_by_id(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<Option<PointsPackage>, CoreError> {
                Ok(None)
            }

            async fn list_points_packages(
                &self,
                _realm_id: &str,
                _enabled_only: bool,
            ) -> Result<Vec<PointsPackage>, CoreError> {
                Ok(vec![])
            }

            async fn update_points_package(
                &self,
                _package: PointsPackage,
            ) -> Result<PointsPackage, CoreError> {
                Ok(PointsPackage {
                    id: Uuid::now_v7(),
                    realm_id: "test-realm".to_string(),
                    name: "test-package".to_string(),
                    title: "Updated Package".to_string(),
                    description: None,
                    points: 500,
                    price: 2999,
                    currency: "USD".to_string(),
                    sort_order: 0,
                    enabled: true,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            }

            async fn delete_points_package(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<(), CoreError> {
                Ok(())
            }

            async fn create_payment_provider_mapping(
                &self,
                _input: CreatePaymentProviderMappingInput,
            ) -> Result<PointsPackagePaymentProvider, CoreError> {
                Ok(PointsPackagePaymentProvider {
                    id: Uuid::now_v7(),
                    points_package_id: Uuid::now_v7(),
                    payment_provider: "wechat".to_string(),
                    enabled: true,
                    external_product_id: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            }

            async fn list_payment_provider_mappings(
                &self,
                _package_id: Uuid,
            ) -> Result<Vec<PointsPackagePaymentProvider>, CoreError> {
                Ok(vec![])
            }

            async fn find_payment_provider_mapping_by_id(
                &self,
                _mapping_id: Uuid,
            ) -> Result<Option<PointsPackagePaymentProvider>, CoreError> {
                Ok(None)
            }

            async fn update_payment_provider_mapping(
                &self,
                _mapping: PointsPackagePaymentProvider,
            ) -> Result<PointsPackagePaymentProvider, CoreError> {
                Ok(PointsPackagePaymentProvider {
                    id: Uuid::now_v7(),
                    points_package_id: Uuid::now_v7(),
                    payment_provider: "wechat".to_string(),
                    enabled: true,
                    external_product_id: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                })
            }

            async fn delete_payment_provider_mapping(
                &self,
                _mapping_id: Uuid,
            ) -> Result<(), CoreError> {
                Ok(())
            }

            async fn has_purchase_records(&self, _package_id: Uuid) -> Result<bool, CoreError> {
                Ok(false)
            }
        }

        let repo = Arc::new(DuplicateNameMockRepository);
        let service = PointsPackageService::new(repo);

        let input = CreatePointsPackageInput {
            realm_id: "test-realm".to_string(),
            name: "test-package".to_string(),
            title: "Test Package".to_string(),
            description: None,
            points: 500,
            price: 2999,
            currency: "USD".to_string(),
            sort_order: None,
            enabled: None,
        };

        let result = service.create_points_package("test-realm", input).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::Conflict(msg) if msg.contains("already exists") => {
                // Expected error type
            }
            _ => panic!("Expected name conflict error"),
        }
    }
    #[tokio::test]
    async fn test_update_points_package_invalid_price() {
        struct UpdateInvalidPriceMockRepository;

        impl PointsPackageRepository for UpdateInvalidPriceMockRepository {
            async fn find_points_package_by_id(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<Option<PointsPackage>, CoreError> {
                Ok(Some(PointsPackage {
                    id: Uuid::now_v7(),
                    realm_id: "test-realm".to_string(),
                    name: "test-package".to_string(),
                    title: "Test Package".to_string(),
                    description: None,
                    points: 500,
                    price: 2999,
                    currency: "USD".to_string(),
                    sort_order: 0,
                    enabled: true,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            }

            async fn update_points_package(
                &self,
                _package: PointsPackage,
            ) -> Result<PointsPackage, CoreError> {
                unreachable!("Should not update with invalid price")
            }

            async fn find_points_package_by_name(
                &self,
                _realm_id: &str,
                _name: &str,
            ) -> Result<Option<PointsPackage>, CoreError> {
                Ok(None)
            }

            async fn create_points_package(
                &self,
                _input: CreatePointsPackageInput,
            ) -> Result<PointsPackage, CoreError> {
                unreachable!()
            }

            async fn list_points_packages(
                &self,
                _realm_id: &str,
                _enabled_only: bool,
            ) -> Result<Vec<PointsPackage>, CoreError> {
                Ok(vec![])
            }

            async fn delete_points_package(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<(), CoreError> {
                Ok(())
            }

            async fn create_payment_provider_mapping(
                &self,
                _input: CreatePaymentProviderMappingInput,
            ) -> Result<PointsPackagePaymentProvider, CoreError> {
                unreachable!()
            }

            async fn list_payment_provider_mappings(
                &self,
                _package_id: Uuid,
            ) -> Result<Vec<PointsPackagePaymentProvider>, CoreError> {
                Ok(vec![])
            }

            async fn find_payment_provider_mapping_by_id(
                &self,
                _mapping_id: Uuid,
            ) -> Result<Option<PointsPackagePaymentProvider>, CoreError> {
                Ok(None)
            }

            async fn update_payment_provider_mapping(
                &self,
                _mapping: PointsPackagePaymentProvider,
            ) -> Result<PointsPackagePaymentProvider, CoreError> {
                unreachable!()
            }

            async fn delete_payment_provider_mapping(
                &self,
                _mapping_id: Uuid,
            ) -> Result<(), CoreError> {
                Ok(())
            }

            async fn has_purchase_records(&self, _package_id: Uuid) -> Result<bool, CoreError> {
                Ok(false)
            }
        }

        let repo = Arc::new(UpdateInvalidPriceMockRepository);
        let service = PointsPackageService::new(repo);

        let package_id = Uuid::now_v7();

        let input = UpdatePointsPackageInput {
            id: package_id,
            realm_id: "test-realm".to_string(),
            price: Some(-100), // Invalid
            title: None,
            description: None,
            currency: None,
            sort_order: None,
            enabled: None,
        };

        let result = service
            .update_points_package("test-realm", package_id, input)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::InvalidPrice(amount) => {
                assert_eq!(amount, -100);
            }
            _ => panic!("Expected InvalidPrice error"),
        }
    }
    #[tokio::test]
    async fn test_delete_points_package_with_purchase_records() {
        struct DeleteWithPurchasesMockRepository;

        impl PointsPackageRepository for DeleteWithPurchasesMockRepository {
            async fn find_points_package_by_id(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<Option<PointsPackage>, CoreError> {
                Ok(Some(PointsPackage {
                    id: Uuid::now_v7(),
                    realm_id: "test-realm".to_string(),
                    name: "test-package".to_string(),
                    title: "Test Package".to_string(),
                    description: None,
                    points: 500,
                    price: 2999,
                    currency: "USD".to_string(),
                    sort_order: 0,
                    enabled: true,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }))
            }

            async fn has_purchase_records(&self, _package_id: Uuid) -> Result<bool, CoreError> {
                Ok(true) // Has purchase records
            }

            async fn delete_points_package(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<(), CoreError> {
                unreachable!("Should not delete with purchase records")
            }

            async fn find_points_package_by_name(
                &self,
                _realm_id: &str,
                _name: &str,
            ) -> Result<Option<PointsPackage>, CoreError> {
                Ok(None)
            }

            async fn create_points_package(
                &self,
                _input: CreatePointsPackageInput,
            ) -> Result<PointsPackage, CoreError> {
                unreachable!()
            }

            async fn list_points_packages(
                &self,
                _realm_id: &str,
                _enabled_only: bool,
            ) -> Result<Vec<PointsPackage>, CoreError> {
                Ok(vec![])
            }

            async fn update_points_package(
                &self,
                _package: PointsPackage,
            ) -> Result<PointsPackage, CoreError> {
                unreachable!()
            }

            async fn create_payment_provider_mapping(
                &self,
                _input: CreatePaymentProviderMappingInput,
            ) -> Result<PointsPackagePaymentProvider, CoreError> {
                unreachable!()
            }

            async fn list_payment_provider_mappings(
                &self,
                _package_id: Uuid,
            ) -> Result<Vec<PointsPackagePaymentProvider>, CoreError> {
                Ok(vec![])
            }

            async fn find_payment_provider_mapping_by_id(
                &self,
                _mapping_id: Uuid,
            ) -> Result<Option<PointsPackagePaymentProvider>, CoreError> {
                Ok(None)
            }

            async fn update_payment_provider_mapping(
                &self,
                _mapping: PointsPackagePaymentProvider,
            ) -> Result<PointsPackagePaymentProvider, CoreError> {
                unreachable!()
            }

            async fn delete_payment_provider_mapping(
                &self,
                _mapping_id: Uuid,
            ) -> Result<(), CoreError> {
                unreachable!()
            }
        }

        let repo = Arc::new(DeleteWithPurchasesMockRepository);
        let service = PointsPackageService::new(repo);

        let result = service
            .delete_points_package("test-realm", Uuid::now_v7())
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::PackageHasPurchaseRecords => {
                // Expected error type
            }
            _ => panic!("Expected PackageHasPurchaseRecords error"),
        }
    }
    #[tokio::test]
    async fn test_add_payment_provider_mapping_duplicate() {
        struct DuplicateMappingMockRepository;

        impl PointsPackageRepository for DuplicateMappingMockRepository {
            async fn list_payment_provider_mappings(
                &self,
                _package_id: Uuid,
            ) -> Result<Vec<PointsPackagePaymentProvider>, CoreError> {
                // Return existing mapping with same provider
                Ok(vec![PointsPackagePaymentProvider {
                    id: Uuid::now_v7(),
                    points_package_id: Uuid::now_v7(),
                    payment_provider: "stripe".to_string(),
                    enabled: true,
                    external_product_id: None,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                }])
            }

            async fn create_payment_provider_mapping(
                &self,
                _input: CreatePaymentProviderMappingInput,
            ) -> Result<PointsPackagePaymentProvider, CoreError> {
                unreachable!("Should not create duplicate mapping")
            }

            async fn find_points_package_by_id(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<Option<PointsPackage>, CoreError> {
                Ok(None)
            }

            async fn find_points_package_by_name(
                &self,
                _realm_id: &str,
                _name: &str,
            ) -> Result<Option<PointsPackage>, CoreError> {
                Ok(None)
            }

            async fn create_points_package(
                &self,
                _input: CreatePointsPackageInput,
            ) -> Result<PointsPackage, CoreError> {
                unreachable!()
            }

            async fn list_points_packages(
                &self,
                _realm_id: &str,
                _enabled_only: bool,
            ) -> Result<Vec<PointsPackage>, CoreError> {
                Ok(vec![])
            }

            async fn update_points_package(
                &self,
                _package: PointsPackage,
            ) -> Result<PointsPackage, CoreError> {
                unreachable!()
            }

            async fn delete_points_package(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<(), CoreError> {
                unreachable!()
            }

            async fn find_payment_provider_mapping_by_id(
                &self,
                _mapping_id: Uuid,
            ) -> Result<Option<PointsPackagePaymentProvider>, CoreError> {
                Ok(None)
            }

            async fn update_payment_provider_mapping(
                &self,
                _mapping: PointsPackagePaymentProvider,
            ) -> Result<PointsPackagePaymentProvider, CoreError> {
                unreachable!()
            }

            async fn delete_payment_provider_mapping(
                &self,
                _mapping_id: Uuid,
            ) -> Result<(), CoreError> {
                unreachable!()
            }

            async fn has_purchase_records(&self, _package_id: Uuid) -> Result<bool, CoreError> {
                Ok(false)
            }
        }

        let repo = Arc::new(DuplicateMappingMockRepository);
        let service = PointsPackageService::new(repo);

        let input = CreatePaymentProviderMappingInput {
            points_package_id: Uuid::now_v7(),
            payment_provider: "stripe".to_string(), // Duplicate
            external_product_id: Some("prod_456".to_string()),
            enabled: true,
        };

        let result = service.add_payment_provider_mapping(input).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::PaymentProviderAlreadyConfigured(provider) => {
                assert_eq!(provider, "stripe");
            }
            _ => panic!("Expected PaymentProviderAlreadyConfigured error"),
        }
    }
}
