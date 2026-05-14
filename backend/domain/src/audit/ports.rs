use std::future::Future;
use uuid::Uuid;

use crate::common::entities::app_errors::CoreError;

use super::entities::{AuditEvent, AuditEventFilters, NewAuditEvent, PaginatedAuditEvents};

#[cfg_attr(test, mockall::automock)]
pub trait AuditEventRepository: Send + Sync {
    fn create(
        &self,
        event: NewAuditEvent,
    ) -> impl Future<Output = Result<AuditEvent, CoreError>> + Send;

    fn list_paginated(
        &self,
        realm_id: &str,
        filters: AuditEventFilters,
    ) -> impl Future<Output = Result<PaginatedAuditEvents, CoreError>> + Send;

    fn find_by_id(
        &self,
        realm_id: &str,
        event_id: Uuid,
    ) -> impl Future<Output = Result<Option<AuditEvent>, CoreError>> + Send;
}
