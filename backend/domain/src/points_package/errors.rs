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

    /// Standard package cannot have original price
    fn standard_cannot_have_original_price() -> Self;

    /// Original price must be greater than selling price
    fn original_price_not_greater_than_selling_price(original: i64, selling: i64) -> Self;

    /// Promotional time range invalid (end must be after start)
    fn promo_time_range_invalid() -> Self;
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

    fn standard_cannot_have_original_price() -> Self {
        tracing::debug!("Standard package cannot have original price");
        CoreError::BadRequest("Standard package cannot have original price".to_string())
    }

    fn original_price_not_greater_than_selling_price(original: i64, selling: i64) -> Self {
        tracing::debug!(
            "Original price ({}) must be greater than selling price ({})",
            original,
            selling
        );
        CoreError::BadRequest(format!(
            "Original price ({}) must be greater than selling price ({})",
            original, selling
        ))
    }

    fn promo_time_range_invalid() -> Self {
        tracing::debug!("Promotional end time must be after start time");
        CoreError::BadRequest("Promotional end time must be after start time".to_string())
    }
}

/// Result type for PointsPackage operations
pub type PointsPackageResult<T> = Result<T, CoreError>;
