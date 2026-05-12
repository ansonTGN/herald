// Points Package-specific error types

use crate::common::entities::app_errors::CoreError;

/// Trait for PointsPackage-specific error extensions
pub trait PointsPackageErrorExt {
    /// Points package not found
    fn package_not_found(package_id: &str) -> Self;

    /// Points package name already exists
    fn name_already_exists(name: &str) -> Self;

    /// Payment provider not configured
    fn payment_provider_not_configured(provider: &str) -> Self;

    /// Payment provider already configured
    fn payment_provider_already_configured(provider: &str) -> Self;

    /// Cannot delete package with purchase records
    fn has_purchase_records() -> Self;

    /// Invalid points amount
    fn invalid_points_amount(points: i64) -> Self;

    /// Invalid price
    fn invalid_price(price: i64) -> Self;
}

/// PointsPackage-specific error variants that extend CoreError
impl PointsPackageErrorExt for CoreError {
    fn package_not_found(package_id: &str) -> Self {
        tracing::debug!("Points package not found: {}", package_id);
        CoreError::PointsPackageNotFound(package_id.to_string())
    }

    fn name_already_exists(name: &str) -> Self {
        tracing::debug!("Points package name already exists: {}", name);
        CoreError::Conflict(format!("Points package name '{}' already exists", name))
    }

    fn payment_provider_not_configured(provider: &str) -> Self {
        tracing::debug!("Payment provider not configured: {}", provider);
        CoreError::BadRequest(format!(
            "Payment provider '{}' not configured for this package",
            provider
        ))
    }

    fn payment_provider_already_configured(provider: &str) -> Self {
        tracing::debug!("Payment provider already configured: {}", provider);
        CoreError::PaymentProviderAlreadyConfigured(provider.to_string())
    }

    fn has_purchase_records() -> Self {
        tracing::debug!("Cannot delete package with purchase records");
        CoreError::PackageHasPurchaseRecords
    }

    fn invalid_points_amount(points: i64) -> Self {
        tracing::debug!("Invalid points amount: {}", points);
        CoreError::InvalidPointsAmount(points)
    }

    fn invalid_price(price: i64) -> Self {
        tracing::debug!("Invalid price: {}", price);
        CoreError::InvalidPrice(price)
    }
}

/// Result type for PointsPackage operations
pub type PointsPackageResult<T> = Result<T, CoreError>;
