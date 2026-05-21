use herald_core::domain::points::entities::{PointsTransaction, TransactionType};

pub fn create_placeholder_transaction(
    user_id: uuid::Uuid,
    realm_id: &str,
    transaction_type: TransactionType,
) -> PointsTransaction {
    let description = format!("Placeholder for {:?}", transaction_type);
    PointsTransaction {
        id: uuid::Uuid::now_v7(),
        wallet_id: uuid::Uuid::now_v7(),
        user_id,
        realm_id: realm_id.to_string(),
        transaction_type,
        amount: 0,
        balance_after: 0,
        topup_balance_after: Some(0),
        subscription_balance_after: Some(0),
        credit_type: None,
        description: Some(description),
        client_app_id: None,
        subscription_id: None,
        external_ref_id: None,
        created_at: chrono::Utc::now(),
    }
}
