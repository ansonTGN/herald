// =============================================================================
// 分页辅助模块
// =============================================================================
//
// 提供通用的分页逻辑，用于处理分页参数和计算。
//
// =============================================================================

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Pagination request parameters
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct PaginationRequest {
    pub page: i64,
    pub page_size: i64,
}

impl PaginationRequest {
    /// 规范化分页参数
    /// - page 不能小于 0
    /// - page_size 必须在 1-100 之间，默认 20
    pub fn normalize(&self) -> NormalizedPagination {
        let page = if self.page < 0 { 0 } else { self.page };
        let page_size = if self.page_size <= 0 {
            20
        } else {
            self.page_size.min(100)
        };

        NormalizedPagination { page, page_size }
    }
}

/// 规范化后的分页参数
#[derive(Debug, Clone, Copy)]
pub struct NormalizedPagination {
    pub page: i64,
    pub page_size: i64,
}

/// 计算总页数
pub fn calculate_total_pages(total_count: i64, page_size: i64) -> i64 {
    if page_size > 0 {
        (total_count + page_size - 1) / page_size
    } else {
        0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaginationMeta {
    #[schema(example = "0")]
    pub page: i64,
    #[schema(example = "20")]
    pub page_size: i64,
    #[schema(example = "42")]
    pub total_count: i64,
    #[schema(example = "3")]
    pub total_pages: i64,
}
