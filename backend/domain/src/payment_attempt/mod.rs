// Payment Attempt domain module - unified payment tracking for initiator-based platforms

pub mod entities;
pub mod errors;
pub mod ports;
pub mod services;

// Re-export commonly used types
pub use entities::{
    PaymentAttempt, PaymentAttemptStatus, PaymentContext, PurchasableTarget, PurchaseHistoryRow,
};
pub use errors::{PaymentAttemptErrorExt, PaymentAttemptResult};
pub use ports::{CreatePaymentAttemptInput, PaymentAttemptRepository, RecordRenewalAttemptInput};
pub use services::PaymentAttemptService;
