// Purchase domain module - unified fulfillment logic for subscriptions and points packages

pub mod entities;
pub mod errors;
pub mod ports;
pub mod repositories;
pub mod services;

// Re-export commonly used types
pub use entities::PointsPackagePurchase;
pub use errors::{PurchaseErrorExt, PurchaseResult};
pub use ports::{FulfillmentResult, FulfillmentService, FulfillmentType, PointsGrant};
pub use repositories::{CreatePointsPackagePurchaseInput, PurchaseRepository};
pub use services::{
    CompletePaymentAttemptInput, CreatedPaymentAttempt, PaymentCompletionSource,
    PreparePaymentAttemptInput, PreparedPaymentAttempt, PurchaseTargetSnapshot,
};
