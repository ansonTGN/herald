// Purchase domain module - unified fulfillment logic for subscriptions and one-time purchases

pub mod entities;
pub mod errors;
pub mod ports;
pub mod repositories;
pub mod services;

// Re-export commonly used types
pub use errors::{PurchaseErrorExt, PurchaseResult};
pub use ports::{FulfillmentResult, FulfillmentService, FulfillmentType, PointsGrant};
pub use services::{
    CompletePaymentAttemptInput, CreatedPaymentAttempt, PaymentCompletionSource,
    PreparePaymentAttemptInput, PreparedPaymentAttempt, PurchaseTargetSnapshot, metadata_keys,
};
