use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use herald_domain::audit::{
    AuditEvent, AuditEventFilters, AuditEventRepository, NewAuditEvent, PaginatedAuditEvents,
};
use herald_domain::common::entities::app_errors::CoreError;
use herald_entity::audit_event;

pub struct PostgresAuditEventRepository {
    db: DatabaseConnection,
}

impl PostgresAuditEventRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

impl AuditEventRepository for PostgresAuditEventRepository {
    async fn create(&self, event: NewAuditEvent) -> Result<AuditEvent, CoreError> {
        let now = chrono::Utc::now();
        let id = Uuid::now_v7();

        let active_model = audit_event::ActiveModel {
            id: Set(id),
            realm_id: Set(event.realm_id),
            category: Set(enum_to_string(&event.category)),
            action: Set(enum_to_string(&event.action)),
            actor_id: Set(event.actor_id),
            actor_type: Set(event.actor_type.as_ref().map(enum_to_string)),
            actor_name: Set(event.actor_name),
            target_type: Set(enum_to_string(&event.target_type)),
            target_id: Set(event.target_id),
            target_name: Set(event.target_name),
            result: Set(enum_to_string(&event.result)),
            details: Set(event.details),
            ip_address: Set(event.ip_address),
            user_agent: Set(event.user_agent),
            trace_id: Set(event.trace_id),
            created_at: Set(now.into()),
        };

        let model = active_model
            .insert(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        to_domain(model)
    }

    async fn list_paginated(
        &self,
        realm_id: &str,
        filters: AuditEventFilters,
    ) -> Result<PaginatedAuditEvents, CoreError> {
        let mut query =
            audit_event::Entity::find().filter(audit_event::Column::RealmId.eq(realm_id));

        if let Some(ref category) = filters.category {
            query = query.filter(audit_event::Column::Category.eq(enum_to_string(category)));
        }
        if let Some(ref action) = filters.action {
            query = query.filter(audit_event::Column::Action.eq(enum_to_string(action)));
        }
        if let Some(ref actor_id) = filters.actor_id {
            query = query.filter(audit_event::Column::ActorId.eq(actor_id.as_str()));
        }
        if let Some(start_time) = filters.start_time {
            query = query.filter(
                audit_event::Column::CreatedAt
                    .gte(sea_orm::prelude::DateTimeWithTimeZone::from(start_time)),
            );
        }
        if let Some(end_time) = filters.end_time {
            query = query.filter(
                audit_event::Column::CreatedAt
                    .lte(sea_orm::prelude::DateTimeWithTimeZone::from(end_time)),
            );
        }

        let page = filters.page;
        let page_size = filters.page_size;
        let offset = page * page_size;

        // Get total count
        let total = query
            .clone()
            .count(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        // Get paginated results
        let items: Vec<AuditEvent> = query
            .order_by_desc(audit_event::Column::CreatedAt)
            .offset(offset)
            .limit(page_size)
            .all(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?
            .into_iter()
            .map(to_domain)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PaginatedAuditEvents {
            items,
            page,
            page_size,
            total,
        })
    }

    async fn find_by_id(
        &self,
        realm_id: &str,
        event_id: Uuid,
    ) -> Result<Option<AuditEvent>, CoreError> {
        let result = audit_event::Entity::find()
            .filter(audit_event::Column::Id.eq(event_id))
            .filter(audit_event::Column::RealmId.eq(realm_id))
            .one(&self.db)
            .await
            .map_err(|e| CoreError::DatabaseError(e.to_string()))?;

        result.map(to_domain).transpose()
    }
}

fn to_domain(model: audit_event::Model) -> Result<AuditEvent, CoreError> {
    Ok(AuditEvent {
        id: model.id,
        realm_id: model.realm_id,
        category: parse_enum(&model.category, "category")?,
        action: parse_enum(&model.action, "action")?,
        actor_id: model.actor_id,
        actor_type: model
            .actor_type
            .as_deref()
            .map(|s| parse_enum(s, "actor_type"))
            .transpose()?,
        actor_name: model.actor_name,
        target_type: parse_enum(&model.target_type, "target_type")?,
        target_id: model.target_id,
        target_name: model.target_name,
        result: parse_enum(&model.result, "result")?,
        details: model.details,
        ip_address: model.ip_address,
        user_agent: model.user_agent,
        trace_id: model.trace_id,
        created_at: chrono::DateTime::from(model.created_at),
    })
}

fn enum_to_string<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string()
}

fn parse_enum<T: serde::de::DeserializeOwned>(s: &str, field: &str) -> Result<T, CoreError> {
    serde_json::from_str(&format!("\"{}\"", s)).map_err(|e| {
        CoreError::InternalServerError(format!("invalid {} value '{}': {}", field, s, e))
    })
}

#[cfg(test)]
mod repository_test;
