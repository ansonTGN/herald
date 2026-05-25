// Points Package domain module - points package catalog and management

pub mod entities;
pub mod errors;
pub mod ports;
pub mod services;

// Re-export commonly used types
pub use entities::{PackageType, PointsPackage, PointsPackagePaymentProvider};
pub use errors::{PointsPackageErrorExt, PointsPackageResult};
pub use ports::{
    CreatePaymentProviderMappingInput, CreatePointsPackageInput, PointsPackageRepository,
    UpdatePaymentProviderMappingInput, UpdatePointsPackageInput,
};
pub use services::PointsPackageService;
