// Points Package domain service

use chrono::Utc;
use std::sync::Arc;

use super::entities::{PackageType, PointsPackage, PointsPackagePaymentProvider};
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

    /// Validate promo-related fields on a points package state.
    /// Used by both create and update flows to ensure consistency.
    fn validate_promo_fields(
        package_type: &PackageType,
        original_price: Option<i64>,
        price: i64,
        promo_start_time: Option<chrono::DateTime<Utc>>,
        promo_end_time: Option<chrono::DateTime<Utc>>,
    ) -> Result<(), CoreError> {
        match package_type {
            PackageType::Standard => {
                if original_price.is_some() {
                    return Err(CoreError::standard_cannot_have_original_price());
                }
            }
            PackageType::Promotional => {
                if let Some(orig) = original_price
                    && orig <= price
                {
                    return Err(CoreError::original_price_not_greater_than_selling_price(
                        orig, price,
                    ));
                }
            }
        }

        if let (Some(start), Some(end)) = (&promo_start_time, &promo_end_time)
            && end <= start
        {
            return Err(CoreError::promo_time_range_invalid());
        }

        Ok(())
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

    async fn ensure_points_permission<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        action: &str,
    ) -> Result<(), CoreError> {
        self.ensure_realm_access(identity, realm_id).await?;

        if !identity.is_user() {
            return Err(CoreError::Forbidden(
                "Access denied: user required".to_string(),
            ));
        }

        let allowed = permission_checker
            .check_permission(realm_id, &identity.user_id(), "points", action)
            .await
            .map_err(|e| CoreError::InternalServerError(format!("Permission check failed: {e}")))?;

        if !allowed {
            return Err(CoreError::Forbidden(format!(
                "Insufficient permissions: points.{action} required"
            )));
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
        self.ensure_points_permission(identity, permission_checker, realm_id, "manage")
            .await?;
        self.create_points_package(realm_id, input).await
    }

    pub async fn list_visible_points_packages<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
    ) -> PointsPackageResult<Vec<PointsPackage>> {
        self.ensure_points_permission(identity, permission_checker, realm_id, "view")
            .await?;
        self.list_points_packages(realm_id, true).await
    }

    pub async fn get_visible_points_package<P: PermissionService>(
        &self,
        identity: &Identity,
        permission_checker: &P,
        realm_id: &str,
        package_id: uuid::Uuid,
    ) -> PointsPackageResult<PointsPackage> {
        self.ensure_points_permission(identity, permission_checker, realm_id, "view")
            .await?;

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
        self.ensure_points_permission(identity, permission_checker, realm_id, "manage")
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
        self.ensure_points_permission(identity, permission_checker, realm_id, "manage")
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
        self.ensure_points_permission(identity, permission_checker, realm_id, "view")
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
        self.ensure_points_permission(identity, permission_checker, realm_id, "manage")
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
        self.ensure_points_permission(identity, permission_checker, realm_id, "manage")
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
        self.ensure_points_permission(identity, permission_checker, realm_id, "manage")
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

        // Validate promo fields
        let effective_package_type = input
            .package_type
            .as_ref()
            .unwrap_or(&PackageType::Standard);
        Self::validate_promo_fields(
            effective_package_type,
            input.original_price,
            input.price,
            input.promo_start_time,
            input.promo_end_time,
        )?;

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

    /// List user-visible packages: enabled only, with expired/not-started promos
    /// filtered out, active promos sorted first.
    pub async fn list_user_visible_packages(
        &self,
        realm_id: &str,
    ) -> PointsPackageResult<Vec<PointsPackage>> {
        let packages = self.list_points_packages(realm_id, true).await?;
        let now = Utc::now();

        let mut visible: Vec<PointsPackage> = packages
            .into_iter()
            .filter(|pkg| match pkg.package_type {
                PackageType::Standard => true,
                PackageType::Promotional => {
                    // Filter out expired promos
                    if pkg.is_promo_expired() {
                        return false;
                    }
                    // Filter out not-yet-started promos
                    if let Some(start) = pkg.promo_start_time
                        && start > now
                    {
                        return false;
                    }
                    true
                }
            })
            .collect();

        // Sort: active promo first -> sort_order desc -> created_at asc
        visible.sort_by(|a, b| {
            // Active promo packages come first
            let a_active = a.is_promo_active();
            let b_active = b.is_promo_active();
            match (a_active, b_active) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    // Within same group: sort_order desc (higher first)
                    match b.sort_order.cmp(&a.sort_order) {
                        std::cmp::Ordering::Equal => {
                            // Then created_at asc (earlier first)
                            a.created_at.cmp(&b.created_at)
                        }
                        other => other,
                    }
                }
            }
        });

        Ok(visible)
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
        if let Some(package_type) = input.package_type {
            package.package_type = package_type;
        }

        // Apply promo fields based on resulting package type
        match package.package_type {
            PackageType::Standard => {
                // When Standard, clear all promo fields regardless of input
                package.original_price = None;
                package.promo_start_time = None;
                package.promo_end_time = None;
            }
            PackageType::Promotional => {
                // Nested Option: Some(Some(v)) sets, Some(None) clears, None leaves unchanged
                if let Some(original_price) = input.original_price {
                    package.original_price = original_price;
                }
                if let Some(promo_start_time) = input.promo_start_time {
                    package.promo_start_time = promo_start_time;
                }
                if let Some(promo_end_time) = input.promo_end_time {
                    package.promo_end_time = promo_end_time;
                }
            }
        }

        // Validate the resulting state for promo consistency
        Self::validate_promo_fields(
            &package.package_type,
            package.original_price,
            package.price,
            package.promo_start_time,
            package.promo_end_time,
        )?;

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
    use super::super::entities::PackageType;
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
                package_type: PackageType::Standard,
                original_price: None,
                promo_start_time: None,
                promo_end_time: None,
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
                package_type: PackageType::Standard,
                original_price: None,
                promo_start_time: None,
                promo_end_time: None,
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
            package_type: None,
            original_price: None,
            promo_start_time: None,
            promo_end_time: None,
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
            package_type: None,
            original_price: None,
            promo_start_time: None,
            promo_end_time: None,
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
                    package_type: PackageType::Standard,
                    original_price: None,
                    promo_start_time: None,
                    promo_end_time: None,
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
                    package_type: PackageType::Standard,
                    original_price: None,
                    promo_start_time: None,
                    promo_end_time: None,
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
            package_type: None,
            original_price: None,
            promo_start_time: None,
            promo_end_time: None,
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
                    package_type: PackageType::Standard,
                    original_price: None,
                    promo_start_time: None,
                    promo_end_time: None,
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
            package_type: None,
            original_price: None,
            promo_start_time: None,
            promo_end_time: None,
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
                    package_type: PackageType::Standard,
                    original_price: None,
                    promo_start_time: None,
                    promo_end_time: None,
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

    // --- Promo validation tests ---

    fn make_test_package(
        package_type: PackageType,
        price: i64,
        original_price: Option<i64>,
        promo_start_time: Option<chrono::DateTime<Utc>>,
        promo_end_time: Option<chrono::DateTime<Utc>>,
    ) -> PointsPackage {
        PointsPackage {
            id: Uuid::now_v7(),
            realm_id: "test-realm".to_string(),
            name: "test-package".to_string(),
            title: "Test Package".to_string(),
            description: None,
            points: 500,
            price,
            currency: "USD".to_string(),
            sort_order: 0,
            enabled: true,
            package_type,
            original_price,
            promo_start_time,
            promo_end_time,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn make_create_input(
        package_type: Option<PackageType>,
        price: i64,
        original_price: Option<i64>,
        promo_start_time: Option<chrono::DateTime<Utc>>,
        promo_end_time: Option<chrono::DateTime<Utc>>,
    ) -> CreatePointsPackageInput {
        CreatePointsPackageInput {
            realm_id: "test-realm".to_string(),
            name: "test-package".to_string(),
            title: "Test Package".to_string(),
            description: None,
            points: 500,
            price,
            currency: "USD".to_string(),
            sort_order: None,
            enabled: None,
            package_type,
            original_price,
            promo_start_time,
            promo_end_time,
        }
    }

    #[tokio::test]
    async fn test_create_promo_package_original_price_not_greater() {
        let repo = Arc::new(MockPointsPackageRepository);
        let service = PointsPackageService::new(repo);

        // original_price == price should fail
        let input = make_create_input(Some(PackageType::Promotional), 2999, Some(2999), None, None);
        let result = service.create_points_package("test-realm", input).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::BadRequest(msg) => {
                assert!(msg.contains("must be greater than selling price"));
            }
            _ => panic!("Expected BadRequest error for original_price <= price"),
        }

        // original_price < price should also fail
        let input = make_create_input(Some(PackageType::Promotional), 2999, Some(1000), None, None);
        let result = service.create_points_package("test-realm", input).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_standard_package_with_original_price() {
        let repo = Arc::new(MockPointsPackageRepository);
        let service = PointsPackageService::new(repo);

        let input = make_create_input(Some(PackageType::Standard), 2999, Some(5000), None, None);
        let result = service.create_points_package("test-realm", input).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::BadRequest(msg) => {
                assert!(msg.contains("Standard package cannot have original price"));
            }
            _ => panic!("Expected BadRequest error for standard with original_price"),
        }
    }

    #[tokio::test]
    async fn test_create_standard_package_default_with_original_price() {
        // When package_type is None (defaults to Standard), original_price should be rejected
        let repo = Arc::new(MockPointsPackageRepository);
        let service = PointsPackageService::new(repo);

        let input = make_create_input(None, 2999, Some(5000), None, None);
        let result = service.create_points_package("test-realm", input).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::BadRequest(msg) => {
                assert!(msg.contains("Standard package cannot have original price"));
            }
            _ => panic!("Expected BadRequest error for default Standard with original_price"),
        }
    }

    #[tokio::test]
    async fn test_create_promo_package_valid() {
        // Valid promo package: original_price > price
        let repo = Arc::new(MockPointsPackageRepository);
        let service = PointsPackageService::new(repo);

        let input = make_create_input(Some(PackageType::Promotional), 2999, Some(5000), None, None);
        let result = service.create_points_package("test-realm", input).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_promo_time_range_invalid() {
        let repo = Arc::new(MockPointsPackageRepository);
        let service = PointsPackageService::new(repo);

        let now = chrono::Utc::now();
        let start = now + chrono::Duration::days(2);
        let end = now + chrono::Duration::days(1);

        let input = make_create_input(
            Some(PackageType::Promotional),
            2999,
            Some(5000),
            Some(start),
            Some(end),
        );
        let result = service.create_points_package("test-realm", input).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CoreError::BadRequest(msg) => {
                assert!(msg.contains("end time must be after start time"));
            }
            _ => panic!("Expected BadRequest error for invalid promo time range"),
        }
    }

    #[tokio::test]
    async fn test_update_standard_to_promo() {
        struct UpdateStandardToPromoMockRepository;

        impl PointsPackageRepository for UpdateStandardToPromoMockRepository {
            async fn find_points_package_by_id(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<Option<PointsPackage>, CoreError> {
                Ok(Some(make_test_package(
                    PackageType::Standard,
                    2999,
                    None,
                    None,
                    None,
                )))
            }

            async fn update_points_package(
                &self,
                package: PointsPackage,
            ) -> Result<PointsPackage, CoreError> {
                // Verify the updated state
                assert_eq!(package.package_type, PackageType::Promotional);
                assert_eq!(package.original_price, Some(5000));
                assert!(package.promo_start_time.is_some());
                assert!(package.promo_end_time.is_some());
                Ok(package)
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

        let repo = Arc::new(UpdateStandardToPromoMockRepository);
        let service = PointsPackageService::new(repo);
        let package_id = Uuid::now_v7();

        let now = chrono::Utc::now();
        let start = now + chrono::Duration::hours(1);
        let end = now + chrono::Duration::days(7);

        let input = UpdatePointsPackageInput {
            id: package_id,
            realm_id: "test-realm".to_string(),
            title: None,
            description: None,
            price: None,
            currency: None,
            sort_order: None,
            enabled: None,
            package_type: Some(PackageType::Promotional),
            original_price: Some(Some(5000)),
            promo_start_time: Some(Some(start)),
            promo_end_time: Some(Some(end)),
        };

        let result = service
            .update_points_package("test-realm", package_id, input)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_promo_to_standard_clears_fields() {
        struct UpdatePromoToStandardMockRepository;

        impl PointsPackageRepository for UpdatePromoToStandardMockRepository {
            async fn find_points_package_by_id(
                &self,
                _realm_id: &str,
                _package_id: Uuid,
            ) -> Result<Option<PointsPackage>, CoreError> {
                let now = chrono::Utc::now();
                Ok(Some(make_test_package(
                    PackageType::Promotional,
                    2999,
                    Some(5000),
                    Some(now),
                    Some(now + chrono::Duration::days(7)),
                )))
            }

            async fn update_points_package(
                &self,
                package: PointsPackage,
            ) -> Result<PointsPackage, CoreError> {
                // Verify promo fields are cleared
                assert_eq!(package.package_type, PackageType::Standard);
                assert_eq!(package.original_price, None);
                assert_eq!(package.promo_start_time, None);
                assert_eq!(package.promo_end_time, None);
                Ok(package)
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

        let repo = Arc::new(UpdatePromoToStandardMockRepository);
        let service = PointsPackageService::new(repo);
        let package_id = Uuid::now_v7();

        // Even though input tries to set original_price, switching to Standard clears it
        let input = UpdatePointsPackageInput {
            id: package_id,
            realm_id: "test-realm".to_string(),
            title: None,
            description: None,
            price: None,
            currency: None,
            sort_order: None,
            enabled: None,
            package_type: Some(PackageType::Standard),
            original_price: Some(Some(9999)), // This should be ignored/cleared
            promo_start_time: None,
            promo_end_time: None,
        };

        let result = service
            .update_points_package("test-realm", package_id, input)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_user_visible_filters_expired_promos() {
        struct ExpiredPromoMockRepository;

        impl PointsPackageRepository for ExpiredPromoMockRepository {
            async fn list_points_packages(
                &self,
                _realm_id: &str,
                _enabled_only: bool,
            ) -> Result<Vec<PointsPackage>, CoreError> {
                let now = chrono::Utc::now();
                Ok(vec![
                    // Standard package -- should be visible
                    make_test_package(PackageType::Standard, 2999, None, None, None),
                    // Expired promo -- should be filtered out
                    make_test_package(
                        PackageType::Promotional,
                        1999,
                        Some(3999),
                        Some(now - chrono::Duration::days(7)),
                        Some(now - chrono::Duration::days(1)),
                    ),
                ])
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

        let repo = Arc::new(ExpiredPromoMockRepository);
        let service = PointsPackageService::new(repo);

        let result = service.list_user_visible_packages("test-realm").await;
        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].package_type, PackageType::Standard);
    }

    #[tokio::test]
    async fn test_list_user_visible_filters_not_started_promos() {
        struct NotStartedPromoMockRepository;

        impl PointsPackageRepository for NotStartedPromoMockRepository {
            async fn list_points_packages(
                &self,
                _realm_id: &str,
                _enabled_only: bool,
            ) -> Result<Vec<PointsPackage>, CoreError> {
                let now = chrono::Utc::now();
                Ok(vec![
                    // Standard package -- should be visible
                    make_test_package(PackageType::Standard, 2999, None, None, None),
                    // Not-yet-started promo -- should be filtered out
                    make_test_package(
                        PackageType::Promotional,
                        1999,
                        Some(3999),
                        Some(now + chrono::Duration::days(1)),
                        Some(now + chrono::Duration::days(7)),
                    ),
                ])
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

        let repo = Arc::new(NotStartedPromoMockRepository);
        let service = PointsPackageService::new(repo);

        let result = service.list_user_visible_packages("test-realm").await;
        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].package_type, PackageType::Standard);
    }

    #[tokio::test]
    async fn test_list_user_visible_sorts_promos_first() {
        struct SortPromosMockRepository;

        impl PointsPackageRepository for SortPromosMockRepository {
            async fn list_points_packages(
                &self,
                _realm_id: &str,
                _enabled_only: bool,
            ) -> Result<Vec<PointsPackage>, CoreError> {
                let now = chrono::Utc::now();
                let mut std1 = make_test_package(PackageType::Standard, 2999, None, None, None);
                std1.sort_order = 10;
                std1.created_at = now - chrono::Duration::days(3);

                let mut promo1 = make_test_package(
                    PackageType::Promotional,
                    1999,
                    Some(3999),
                    Some(now - chrono::Duration::hours(1)),
                    Some(now + chrono::Duration::days(7)),
                );
                promo1.sort_order = 5;

                let mut std2 = make_test_package(PackageType::Standard, 4999, None, None, None);
                std2.sort_order = 5;
                std2.created_at = now - chrono::Duration::days(1);

                Ok(vec![std1, promo1, std2])
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

        let repo = Arc::new(SortPromosMockRepository);
        let service = PointsPackageService::new(repo);

        let result = service.list_user_visible_packages("test-realm").await;
        assert!(result.is_ok());
        let packages = result.unwrap();
        assert_eq!(packages.len(), 3);

        // Active promo should be first
        assert_eq!(packages[0].package_type, PackageType::Promotional);

        // Among standard packages, higher sort_order first
        assert_eq!(packages[1].package_type, PackageType::Standard);
        assert_eq!(packages[1].sort_order, 10);
        assert_eq!(packages[2].package_type, PackageType::Standard);
        assert_eq!(packages[2].sort_order, 5);
    }
}
