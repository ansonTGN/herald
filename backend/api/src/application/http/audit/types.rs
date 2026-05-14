use herald_core::domain::audit::AuditEvent;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

fn enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string()
}

impl AuditEventResponse {
    pub fn from_event(event: AuditEvent) -> Self {
        Self {
            id: event.id.to_string(),
            category: enum_to_string(&event.category),
            action: enum_to_string(&event.action),
            actor_id: event.actor_id,
            actor_type: event.actor_type.map(|t| enum_to_string(&t)),
            actor_name: event.actor_name,
            target_type: enum_to_string(&event.target_type),
            target_id: event.target_id,
            target_name: event.target_name,
            result: enum_to_string(&event.result),
            ip_address: event.ip_address,
            created_at: event.created_at.to_rfc3339(),
        }
    }
}

impl AuditEventDetailResponse {
    pub fn from_event(event: AuditEvent) -> Self {
        Self {
            id: event.id.to_string(),
            category: enum_to_string(&event.category),
            action: enum_to_string(&event.action),
            actor_id: event.actor_id,
            actor_type: event.actor_type.map(|t| enum_to_string(&t)),
            actor_name: event.actor_name,
            target_type: enum_to_string(&event.target_type),
            target_id: event.target_id,
            target_name: event.target_name,
            result: enum_to_string(&event.result),
            details: event.details,
            ip_address: event.ip_address,
            user_agent: event.user_agent,
            trace_id: event.trace_id,
            created_at: event.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventResponse {
    pub id: String,
    pub category: String,
    pub action: String,
    pub actor_id: String,
    pub actor_type: Option<String>,
    pub actor_name: Option<String>,
    pub target_type: String,
    pub target_id: String,
    pub target_name: Option<String>,
    pub result: String,
    pub ip_address: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventDetailResponse {
    pub id: String,
    pub category: String,
    pub action: String,
    pub actor_id: String,
    pub actor_type: Option<String>,
    pub actor_name: Option<String>,
    pub target_type: String,
    pub target_id: String,
    pub target_name: Option<String>,
    pub result: String,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub trace_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventListResponse {
    pub items: Vec<AuditEventResponse>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEventQueryParams {
    pub category: Option<String>,
    pub action: Option<String>,
    pub actor_id: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    #[serde(default)]
    pub page: Option<u64>,
    #[serde(default)]
    pub page_size: Option<u64>,
}
