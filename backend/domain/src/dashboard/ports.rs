use std::future::Future;

use crate::common::entities::app_errors::CoreError;

use super::entities::DashboardStats;

#[cfg_attr(test, mockall::automock)]
pub trait DashboardRepository: Send + Sync {
    fn get_stats(
        &self,
        realm_id: &str,
    ) -> impl Future<Output = Result<DashboardStats, CoreError>> + Send;
}
