// Core modules
pub mod admin; // Only admin::user module remains, admin::api moved to api crate
pub mod config;
#[cfg(test)]
mod tests;
pub mod third;

// Hexagonal architecture modules
pub mod application;
pub use herald_domain as domain;
pub use herald_entity as entity;
pub use herald_infra as infrastructure;

// Re-export commonly used types
pub use config::PermissionConfig;
pub use infrastructure::billing::{
    IronPressInvoicePdfGenerator, PostgresBillingRepository, PostgresInvoiceRepository,
    decrypt_secret, encrypt_secret,
};
pub use infrastructure::creem::CreemClient;
pub use infrastructure::oauth::ReqwestHttpClient;
pub use infrastructure::payment_attempt::PostgresPaymentAttemptRepository;
pub use infrastructure::points::PostgresPointsRepository;
pub use infrastructure::points_package::PostgresPointsPackageRepository;
pub use infrastructure::purchase::{PostgresFulfillmentService, PostgresPurchaseRepository};
pub use infrastructure::stripe::StripeClient;
pub use infrastructure::webhook::{IdempotencyResult, WebhookEventRepository};
pub use third::email::{EmailConfigStatus, EmailService};
