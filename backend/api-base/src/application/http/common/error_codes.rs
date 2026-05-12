// Numeric error code constants used by API responses.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    InternalError,
    MissingApiKey,
    InvalidApiKey,
    ApiKeyDisabled,
    ApiKeyExpired,
    CrossRealmAccessForbidden,
    InvalidUserIdFormat,
    MissingUserId,
    InvalidClientAppIdFormat,
    InvalidAmount,
    InsufficientPoints,
    AccountFrozenOrClosed,
    InvalidTransactionIdFormat,
    IdempotencyConflict,
    ConcurrentModification,
    RateLimitExceeded,
    AccountNotFound,
    TransactionNotFound,
    ClientAppNotFound,
    SubscriptionNotFound,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Unauthorized => "unauthorized",
            ErrorCode::Forbidden => "forbidden",
            ErrorCode::InternalError => "internal_error",
            ErrorCode::MissingApiKey => "missing_api_key",
            ErrorCode::InvalidApiKey => "invalid_api_key",
            ErrorCode::ApiKeyDisabled => "api_key_disabled",
            ErrorCode::ApiKeyExpired => "api_key_expired",
            ErrorCode::CrossRealmAccessForbidden => "cross_realm_access_forbidden",
            ErrorCode::InvalidUserIdFormat => "invalid_user_id_format",
            ErrorCode::MissingUserId => "missing_user_id",
            ErrorCode::InvalidClientAppIdFormat => "invalid_client_app_id_format",
            ErrorCode::InvalidAmount => "invalid_amount",
            ErrorCode::InsufficientPoints => "insufficient_points",
            ErrorCode::AccountFrozenOrClosed => "account_frozen_or_closed",
            ErrorCode::InvalidTransactionIdFormat => "invalid_transaction_id_format",
            ErrorCode::IdempotencyConflict => "idempotency_conflict",
            ErrorCode::ConcurrentModification => "concurrent_modification",
            ErrorCode::RateLimitExceeded => "rate_limit_exceeded",
            ErrorCode::AccountNotFound => "account_not_found",
            ErrorCode::TransactionNotFound => "transaction_not_found",
            ErrorCode::ClientAppNotFound => "client_app_not_found",
            ErrorCode::SubscriptionNotFound => "subscription_not_found",
        }
    }

    pub const fn as_u32(self) -> u32 {
        match self {
            ErrorCode::Unauthorized => 401,
            ErrorCode::Forbidden => 403,
            ErrorCode::InternalError => 500,
            ErrorCode::InvalidUserIdFormat => 10001,
            ErrorCode::InvalidClientAppIdFormat => 10002,
            ErrorCode::InvalidTransactionIdFormat => 10003,
            ErrorCode::MissingUserId => 10004,
            ErrorCode::InvalidAmount => 10005,
            ErrorCode::MissingApiKey => 30000,
            ErrorCode::InvalidApiKey => 30001,
            ErrorCode::ApiKeyDisabled => 30002,
            ErrorCode::ApiKeyExpired => 30003,
            ErrorCode::CrossRealmAccessForbidden => 30004,
            ErrorCode::AccountNotFound => 20001,
            ErrorCode::TransactionNotFound => 20002,
            ErrorCode::InsufficientPoints => 20003,
            ErrorCode::AccountFrozenOrClosed => 20004,
            ErrorCode::IdempotencyConflict => 20005,
            ErrorCode::ConcurrentModification => 20007,
            ErrorCode::ClientAppNotFound => 30005,
            ErrorCode::SubscriptionNotFound => 20006,
            ErrorCode::RateLimitExceeded => 42901,
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Currency constant for points system
pub const POINTS_CURRENCY: &str = "points";
