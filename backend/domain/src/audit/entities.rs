use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event_types::{ActorType, AuditAction, AuditCategory, AuditResult, AuditTargetType};

/// A persisted audit event record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub realm_id: String,
    pub category: AuditCategory,
    pub action: AuditAction,
    pub actor_id: String,
    pub actor_type: Option<ActorType>,
    pub actor_name: Option<String>,
    pub target_type: AuditTargetType,
    pub target_id: String,
    pub target_name: Option<String>,
    pub result: AuditResult,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Data needed to create a new audit event (without id and created_at).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAuditEvent {
    pub realm_id: String,
    pub category: AuditCategory,
    pub action: AuditAction,
    pub actor_id: String,
    pub actor_type: Option<ActorType>,
    pub actor_name: Option<String>,
    pub target_type: AuditTargetType,
    pub target_id: String,
    pub target_name: Option<String>,
    pub result: AuditResult,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub trace_id: Option<String>,
}

/// Filters for querying audit events with pagination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventFilters {
    pub category: Option<AuditCategory>,
    pub action: Option<AuditAction>,
    pub actor_id: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    #[serde(default)]
    pub page: u64,
    #[serde(default = "default_page_size")]
    pub page_size: u64,
}

fn default_page_size() -> u64 {
    20
}

impl Default for AuditEventFilters {
    fn default() -> Self {
        Self {
            category: None,
            action: None,
            actor_id: None,
            start_time: None,
            end_time: None,
            page: 0,
            page_size: default_page_size(),
        }
    }
}

/// A paginated list of audit events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedAuditEvents {
    pub items: Vec<AuditEvent>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_audit_event_construction() {
        let event = NewAuditEvent {
            realm_id: "test-realm".to_string(),
            category: AuditCategory::Auth,
            action: AuditAction::AuthLogin,
            actor_id: "user-1".to_string(),
            actor_type: Some(ActorType::User),
            actor_name: Some("alice".to_string()),
            target_type: AuditTargetType::Session,
            target_id: "session-1".to_string(),
            target_name: None,
            result: AuditResult::Success,
            details: Some(serde_json::json!({ "method": "password" })),
            ip_address: Some("127.0.0.1".to_string()),
            user_agent: Some("test-agent".to_string()),
            trace_id: None,
        };

        assert_eq!(event.realm_id, "test-realm");
        assert_eq!(event.category, AuditCategory::Auth);
        assert_eq!(event.action, AuditAction::AuthLogin);
        assert_eq!(event.result, AuditResult::Success);
    }

    #[test]
    fn filters_default_values() {
        let filters = AuditEventFilters::default();
        assert!(filters.category.is_none());
        assert!(filters.action.is_none());
        assert!(filters.actor_id.is_none());
        assert!(filters.start_time.is_none());
        assert!(filters.end_time.is_none());
        assert_eq!(filters.page, 0);
        assert_eq!(filters.page_size, 20);
    }

    #[test]
    fn paginated_audit_events_construction() {
        let page = PaginatedAuditEvents {
            items: vec![],
            page: 0,
            page_size: 20,
            total: 0,
        };
        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
    }

    #[test]
    fn audit_event_serialization_roundtrip() {
        let now = Utc::now();
        let id = Uuid::now_v7();
        let event = AuditEvent {
            id,
            realm_id: "r1".to_string(),
            category: AuditCategory::UserManagement,
            action: AuditAction::UserCreate,
            actor_id: "a1".to_string(),
            actor_type: Some(ActorType::Admin),
            actor_name: Some("admin".to_string()),
            target_type: AuditTargetType::User,
            target_id: "t1".to_string(),
            target_name: Some("bob".to_string()),
            result: AuditResult::Success,
            details: None,
            ip_address: None,
            user_agent: None,
            trace_id: None,
            created_at: now,
        };

        let json = serde_json::to_string(&event).unwrap();
        let back: AuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, id);
        assert_eq!(back.realm_id, "r1");
        assert_eq!(back.category, AuditCategory::UserManagement);
        assert_eq!(back.action, AuditAction::UserCreate);
    }
}
