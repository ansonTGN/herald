// Centralized security and operational defaults for the Herald backend.
// All crates should import from here rather than defining local constants.

// --- Session ---
pub const DEFAULT_SESSION_TTL_SECONDS: u64 = 1800;
pub const DEFAULT_OAUTH_SESSION_TTL_SECONDS: u64 = 600;
pub const DEFAULT_TOTP_TEMP_SESSION_TTL_SECONDS: u64 = 300;

// --- TOTP ---
pub const TOTP_MAX_FAILURES: i64 = 5;
pub const TOTP_LOCKOUT_SECONDS: u64 = 900;

// --- Rate limits: (max_requests, window_seconds) ---
pub const LOGIN_IP_RATE_LIMIT: (i64, usize) = (10, 60);
pub const LOGIN_IDENTIFIER_RATE_LIMIT: (i64, usize) = (2, 60);

pub const REGISTER_IP_RATE_LIMIT: (i64, usize) = (5, 60);
pub const REGISTER_EMAIL_RATE_LIMIT: (i64, usize) = (5, 60);

pub const RESET_PASSWORD_REQUEST_IP_RATE_LIMIT: (i64, usize) = (5, 60);
pub const RESET_PASSWORD_REQUEST_EMAIL_RATE_LIMIT: (i64, usize) = (5, 60);
pub const RESET_PASSWORD_CONFIRM_IP_RATE_LIMIT: (i64, usize) = (5, 60);

pub const TOTP_VERIFY_USER_RATE_LIMIT: (i64, usize) = (5, 60);
pub const TOTP_VERIFY_IP_RATE_LIMIT: (i64, usize) = (10, 60);

// --- Password ---
pub const DEFAULT_BCRYPT_COST: u32 = 10;

// --- HTTP ---
pub const DEFAULT_HTTP_CLIENT_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_HTTP_CLIENT_CONNECT_TIMEOUT_SECS: u64 = 10;

// --- OAuth ---
pub const OAUTH_STATE_TTL_SECONDS: u64 = 300;
pub const OAUTH_STATE_VALIDATION_TIMEOUT_SECONDS: i64 = 300;

// --- JWT ---
pub const DEFAULT_JWT_EXPIRATION_SECONDS: i64 = 7 * 24 * 60 * 60;

// --- Device Code ---
pub const DEVICE_CODE_TTL_SECONDS: u64 = 900;
pub const DEVICE_CODE_DEFAULT_INTERVAL_SECONDS: i64 = 5;
pub const DEVICE_CODE_SLOW_DOWN_INCREMENT_SECONDS: i64 = 5;
pub const DEVICE_CODE_USER_CODE_LENGTH: usize = 8;
pub const DEVICE_CODE_USER_CODE_ALPHABET: &str = "BCDFGHJKMNPQRSTVWXYZ";
