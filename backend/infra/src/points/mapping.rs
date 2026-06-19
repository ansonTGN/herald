// Mapping between SeaORM entities and Points domain entities
//
// This module provides conversion functions between database entities (SeaORM models)
// and domain entities. This isolates the infrastructure layer from the domain layer,
// maintaining clean architecture boundaries.

use sea_orm::ActiveValue::Set;

use herald_domain::points::entities::{
    CreditLedgerStatus, CreditSourceType, CreditType, PointsConsumptionAllocation,
    PointsCreditLedger, PointsRevocationRecord, RevocationType,
};
use herald_entity::{
    points_consumption_allocation, points_credit_ledger, points_revocation_record,
};

// ===== From SeaORM Model to Domain Entity =====

/// Convert SeaORM points_credit_ledger Model to domain PointsCreditLedger
pub fn points_credit_ledger_from_model(model: points_credit_ledger::Model) -> PointsCreditLedger {
    PointsCreditLedger {
        id: model.id,
        user_id: model.user_id,
        realm_id: model.realm_id,
        bucket_id: Some(model.bucket_id),
        credit_type: parse_enum_with_default(
            &model.credit_type,
            "credit_type",
            CreditType::TopupCredit,
        ),
        source_type: parse_enum_with_default(
            &model.source_type,
            "source_type",
            CreditSourceType::Topup,
        ),
        source_id: model.source_id,
        granted_amount: model.granted_amount,
        used_amount: model.used_amount,
        revoked_amount: model.revoked_amount,
        remaining_amount: model.remaining_amount,
        expires_at: model.expires_at.map(chrono::DateTime::from),
        status: parse_enum_with_default(&model.status, "status", CreditLedgerStatus::Active),
        created_at: chrono::DateTime::from(model.created_at),
        updated_at: chrono::DateTime::from(model.updated_at),
    }
}

/// Convert SeaORM points_consumption_allocation Model to domain PointsConsumptionAllocation
pub fn points_consumption_allocation_from_model(
    model: points_consumption_allocation::Model,
) -> PointsConsumptionAllocation {
    PointsConsumptionAllocation {
        id: model.id,
        transaction_id: model.transaction_id,
        ledger_id: model.ledger_id,
        wallet_id: Some(model.wallet_id),
        user_id: model.user_id,
        realm_id: model.realm_id,
        bucket_id: Some(model.bucket_id),
        allocated_amount: model.allocated_amount,
        ledger_remaining_after: model.ledger_remaining_after,
        created_at: chrono::DateTime::from(model.created_at),
    }
}

/// Convert SeaORM points_revocation_record Model to domain PointsRevocationRecord
pub fn points_revocation_record_from_model(
    model: points_revocation_record::Model,
) -> PointsRevocationRecord {
    PointsRevocationRecord {
        id: model.id,
        ledger_id: model.ledger_id,
        user_id: model.user_id,
        realm_id: model.realm_id,
        revocation_type: parse_enum_with_default(
            &model.revocation_type,
            "revocation_type",
            RevocationType::ExpireRevoke,
        ),
        revoked_amount: model.revoked_amount,
        reason: model.reason,
        reference_id: model.reference_id,
        created_at: chrono::DateTime::from(model.created_at),
    }
}

// ===== From Domain Entity to SeaORM ActiveModel =====

/// Convert domain PointsCreditLedger to SeaORM ActiveModel
pub fn points_credit_ledger_to_active_model(
    domain: &PointsCreditLedger,
) -> points_credit_ledger::ActiveModel {
    points_credit_ledger::ActiveModel {
        id: Set(domain.id),
        user_id: Set(domain.user_id),
        realm_id: Set(domain.realm_id.clone()),
        bucket_id: Set(domain
            .bucket_id
            .expect("bucket_id is required for credit ledger persistence")),
        credit_type: Set(domain.credit_type.to_string()),
        source_type: Set(domain.source_type.to_string()),
        source_id: Set(domain.source_id.clone()),
        granted_amount: Set(domain.granted_amount),
        used_amount: Set(domain.used_amount),
        revoked_amount: Set(domain.revoked_amount),
        remaining_amount: sea_orm::ActiveValue::NotSet, // GENERATED ALWAYS AS column
        expires_at: Set(domain.expires_at.map(|dt| dt.into())),
        status: Set(domain.status.to_string()),
        created_at: Set(domain.created_at.into()),
        updated_at: Set(domain.updated_at.into()),
    }
}

/// Convert domain PointsConsumptionAllocation to SeaORM ActiveModel
pub fn points_consumption_allocation_to_active_model(
    domain: &PointsConsumptionAllocation,
) -> points_consumption_allocation::ActiveModel {
    points_consumption_allocation::ActiveModel {
        id: Set(domain.id),
        transaction_id: Set(domain.transaction_id),
        ledger_id: Set(domain.ledger_id),
        wallet_id: Set(domain
            .wallet_id
            .expect("wallet_id is required for allocation persistence")),
        user_id: Set(domain.user_id),
        realm_id: Set(domain.realm_id.clone()),
        bucket_id: Set(domain
            .bucket_id
            .expect("bucket_id is required for allocation persistence")),
        allocated_amount: Set(domain.allocated_amount),
        ledger_remaining_after: Set(domain.ledger_remaining_after),
        created_at: Set(domain.created_at.into()),
    }
}

/// Convert domain PointsRevocationRecord to SeaORM ActiveModel
pub fn points_revocation_record_to_active_model(
    domain: &PointsRevocationRecord,
) -> points_revocation_record::ActiveModel {
    points_revocation_record::ActiveModel {
        id: Set(domain.id),
        ledger_id: Set(domain.ledger_id),
        user_id: Set(domain.user_id),
        realm_id: Set(domain.realm_id.clone()),
        revocation_type: Set(domain.revocation_type.to_string()),
        revoked_amount: Set(domain.revoked_amount),
        reason: Set(domain.reason.clone()),
        reference_id: Set(domain.reference_id.clone()),
        created_at: Set(domain.created_at.into()),
    }
}

// ===== Helper Functions =====

/// Helper for parsing string enums with logging and a default fallback
/// Used in entity conversion from database
fn parse_enum_with_default<T>(s: &str, field_name: &str, default: T) -> T
where
    T: std::str::FromStr<Err = herald_domain::common::entities::app_errors::CoreError>,
{
    s.parse().unwrap_or_else(|_| {
        tracing::error!("Invalid {}: {}", field_name, s);
        default
    })
}
