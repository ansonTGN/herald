// Points Package repository ports

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::entities::{PackageType, PointsPackage, PointsPackagePaymentProvider};
use crate::common::entities::app_errors::CoreError;

/// Input for creating a points package
#[derive(Debug, Clone)]
pub struct CreatePointsPackageInput {
    pub realm_id: String,
    pub name: String,
    pub title: String,
    pub description: Option<String>,
    pub points: i64,
    pub price: i64,
    pub currency: String,
    pub sort_order: Option<i32>,
    pub enabled: Option<bool>,
    pub package_type: Option<PackageType>,
    pub original_price: Option<i64>,
    pub promo_start_time: Option<DateTime<Utc>>,
    pub promo_end_time: Option<DateTime<Utc>>,
}

/// Input for updating a points package
#[derive(Debug, Clone)]
pub struct UpdatePointsPackageInput {
    pub id: Uuid,
    pub realm_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub price: Option<i64>,
    pub currency: Option<String>,
    pub sort_order: Option<i32>,
    pub enabled: Option<bool>,
    pub package_type: Option<PackageType>,
    pub original_price: Option<Option<i64>>,
    pub promo_start_time: Option<Option<DateTime<Utc>>>,
    pub promo_end_time: Option<Option<DateTime<Utc>>>,
}

/// Input for creating a payment provider mapping
#[derive(Debug, Clone)]
pub struct CreatePaymentProviderMappingInput {
    pub points_package_id: Uuid,
    pub payment_provider: String,
    pub external_product_id: Option<String>,
    pub enabled: bool,
}

/// Input for updating a payment provider mapping
#[derive(Debug, Clone)]
pub struct UpdatePaymentProviderMappingInput {
    pub id: Uuid,
    pub external_product_id: Option<String>,
    pub enabled: bool,
}

/// Repository trait for PointsPackage operations
#[allow(async_fn_in_trait)]
pub trait PointsPackageRepository: Send + Sync {
    /// Create a new points package
    async fn create_points_package(
        &self,
        input: CreatePointsPackageInput,
    ) -> Result<PointsPackage, CoreError>;

    /// Find a points package by ID
    async fn find_points_package_by_id(
        &self,
        realm_id: &str,
        package_id: Uuid,
    ) -> Result<Option<PointsPackage>, CoreError>;

    /// Find a points package by name
    async fn find_points_package_by_name(
        &self,
        realm_id: &str,
        name: &str,
    ) -> Result<Option<PointsPackage>, CoreError>;

    /// List all points packages in a realm
    async fn list_points_packages(
        &self,
        realm_id: &str,
        enabled_only: bool,
    ) -> Result<Vec<PointsPackage>, CoreError>;

    /// Update a points package
    async fn update_points_package(
        &self,
        package: PointsPackage,
    ) -> Result<PointsPackage, CoreError>;

    /// Delete a points package
    async fn delete_points_package(
        &self,
        realm_id: &str,
        package_id: Uuid,
    ) -> Result<(), CoreError>;

    /// Create a payment provider mapping
    async fn create_payment_provider_mapping(
        &self,
        input: CreatePaymentProviderMappingInput,
    ) -> Result<PointsPackagePaymentProvider, CoreError>;

    /// List payment provider mappings for a package
    async fn list_payment_provider_mappings(
        &self,
        package_id: Uuid,
    ) -> Result<Vec<PointsPackagePaymentProvider>, CoreError>;

    /// Find a payment provider mapping by ID
    async fn find_payment_provider_mapping_by_id(
        &self,
        mapping_id: Uuid,
    ) -> Result<Option<PointsPackagePaymentProvider>, CoreError>;

    /// Update a payment provider mapping
    async fn update_payment_provider_mapping(
        &self,
        mapping: PointsPackagePaymentProvider,
    ) -> Result<PointsPackagePaymentProvider, CoreError>;

    /// Delete a payment provider mapping
    async fn delete_payment_provider_mapping(&self, mapping_id: Uuid) -> Result<(), CoreError>;

    /// Check if a package has any purchase records
    async fn has_purchase_records(&self, package_id: Uuid) -> Result<bool, CoreError>;
}
