//! Webhook application service module
//!
//! This module provides high-level webhook processing services with
//! built-in idempotency handling and transaction management.

pub mod service;

pub use service::{WebhookContext, WebhookProcessResult, WebhookService};
