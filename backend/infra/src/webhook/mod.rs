//! Webhook infrastructure module
//!
//! This module provides specialized infrastructure for webhook processing,
//! including event repositories with idempotency handling and transaction management.

pub mod event_repository;

pub use event_repository::{IdempotencyResult, WebhookEventRepository};
