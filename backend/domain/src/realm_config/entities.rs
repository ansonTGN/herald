use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Realm 泛化配置实体
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq)]
pub struct RealmConfig {
    /// Configuration entry UUID
    pub id: Uuid,
    /// Realm this configuration belongs to
    #[serde(rename = "realmId")]
    pub realm_id: String,
    /// Configuration type (totp, turnstile, registration)
    #[serde(rename = "configType")]
    pub config_type: ConfigType,
    /// Configuration key (specific to each config_type)
    #[serde(rename = "configKey")]
    pub config_key: String,
    /// Configuration value (format depends on config_key)
    #[serde(rename = "configValue")]
    pub config_value: String,
    /// Whether this value contains sensitive data (e.g., API keys)
    #[serde(rename = "isSecret")]
    pub is_secret: bool,
    /// Whether this configuration is currently active
    pub enabled: bool,
    /// Additional metadata (JSON object)
    pub metadata: Option<serde_json::Value>,
    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 配置类型枚举
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConfigType {
    /// TOTP 二次认证配置
    ///
    /// Configuration is stored as a JSON object in config_value with the following structure:
    /// - config_key: `settings` (fixed key for TOTP configuration)
    /// - config_value: JSON string with `{"enabled": boolean, "force_enabled": boolean}`
    /// - enabled: boolean (whether the config entry itself is active)
    /// - metadata: null (not used for TOTP)
    ///
    /// Example configuration:
    /// ```json
    /// {
    ///   "config_type": "totp",
    ///   "config_key": "settings",
    ///   "config_value": "{\"enabled\":true,\"force_enabled\":false}",
    ///   "is_secret": false,
    ///   "enabled": true,
    ///   "metadata": null
    /// }
    /// ```
    Totp,

    /// Turnstile 验证码配置
    ///
    /// Valid config_key values:
    /// - `site_key`: Cloudflare Turnstile site key (public, non-secret)
    /// - `secret_key`: Cloudflare Turnstile secret key (secret, mark is_secret=true)
    ///
    /// Example site key configuration:
    /// ```json
    /// {
    ///   "config_type": "turnstile",
    ///   "config_key": "site_key",
    ///   "config_value": "0x4AAAAAAxxxxxxxxxxxxxxxxxx",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example secret key configuration:
    /// ```json
    /// {
    ///   "config_type": "turnstile",
    ///   "config_key": "secret_key",
    ///   "config_value": "0x4AAAAAAxxxxxxxxxxxxxxxxxx",
    ///   "is_secret": true,
    ///   "enabled": true
    /// }
    /// ```
    Turnstile,

    /// 用户注册配置
    ///
    /// Valid config_key values:
    /// - `enabled`: Enable user registration for the realm ("true" or "false")
    /// - `allowed_domains`: Comma-separated list of allowed email domains (e.g., "example.com,test.org")
    /// - `require_email_verification`: Require email verification for new accounts ("true" or "false")
    /// - `password_min_length`: Minimum password length
    /// - `require_uppercase`: Require at least one uppercase letter ("true" or "false")
    /// - `require_lowercase`: Require at least one lowercase letter ("true" or "false")
    /// - `require_numbers`: Require at least one number ("true" or "false")
    /// - `require_special_chars`: Require at least one special character ("true" or "false")
    ///
    /// Example allowed domains configuration:
    /// ```json
    /// {
    ///   "config_type": "registration",
    ///   "config_key": "allowed_domains",
    ///   "config_value": "example.com,test.org",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example email verification configuration:
    /// ```json
    /// {
    ///   "config_type": "registration",
    ///   "config_key": "require_email_verification",
    ///   "config_value": "true",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    Registration,

    /// Realm TOTP 加密密钥配置
    ///
    /// This config type stores the realm-level AES-256 key used to encrypt user TOTP secrets.
    ///
    /// Valid config_key values:
    /// - `version_1`: The realm's TOTP encryption key (version 1)
    ///
    /// Configuration details:
    /// - config_key: `version_1` (fixed key for version 1)
    /// - config_value: Base64-encoded 32-byte AES-256 key
    /// - is_secret: true (marked as sensitive)
    /// - enabled: true (key is active)
    /// - metadata: `{"version": 1}` (version metadata)
    ///
    /// Note: Key rotation is NOT implemented. The key_version field in user_totp_config
    /// is reserved for future extension. Currently all keys are version 1.
    ///
    /// Example configuration:
    /// ```json
    /// {
    ///   "config_type": "totp_key",
    ///   "config_key": "version_1",
    ///   "config_value": "SGVsbG8gV29ybGQ=", // Base64-encoded 32-byte key
    ///   "is_secret": true,
    ///   "enabled": true,
    ///   "metadata": {"version": 1}
    /// }
    /// ```
    TotpKey,

    /// Creem 支付提供商配置
    ///
    /// Valid config_key values:
    /// - `api_key`: Creem API key (secret, mark is_secret=true)
    /// - `timeout`: HTTP request timeout in seconds (non-secret, e.g., "30")
    ///
    /// Example API key configuration:
    /// ```json
    /// {
    ///   "config_type": "creem",
    ///   "config_key": "api_key",
    ///   "config_value": "creem_your_api_key_here",
    ///   "is_secret": true,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example timeout configuration:
    /// ```json
    /// {
    ///   "config_type": "creem",
    ///   "config_key": "timeout",
    ///   "config_value": "30",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    Creem,

    /// Stripe 支付提供商配置
    ///
    /// Valid config_key values:
    /// - `api_key`: Stripe API key (secret, mark is_secret=true)
    /// - `webhook_secret`: Stripe webhook signing secret (secret, mark is_secret=true)
    /// - `publishable_key`: Stripe publishable key (non-secret)
    /// - `account_id`: Stripe account ID (non-secret, optional)
    /// - `timeout`: HTTP request timeout in seconds (non-secret, e.g., "30")
    /// - `webhook_endpoint_id`: Stripe webhook endpoint ID (non-secret, for verification)
    ///
    /// Example API key configuration:
    /// ```json
    /// {
    ///   "config_type": "stripe",
    ///   "config_key": "api_key",
    ///   "config_value": "sk_test_your_api_key_here",
    ///   "is_secret": true,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example webhook secret configuration:
    /// ```json
    /// {
    ///   "config_type": "stripe",
    ///   "config_key": "webhook_secret",
    ///   "config_value": "whsec_your_webhook_secret_here",
    ///   "is_secret": true,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example publishable key configuration:
    /// ```json
    /// {
    ///   "config_type": "stripe",
    ///   "config_key": "publishable_key",
    ///   "config_value": "pk_test_your_publishable_key_here",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example timeout configuration:
    /// ```json
    /// {
    ///   "config_type": "stripe",
    ///   "config_key": "timeout",
    ///   "config_value": "30",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    Stripe,

    /// Shopify 支付提供商配置
    ///
    /// Valid config_key values:
    /// - `shop_domain`: Shop 域名（非敏感）
    /// - `admin_access_token`: Admin API token（敏感）
    /// - `storefront_access_token`: Storefront API token（敏感）
    /// - `app_client_secret`: Webhook HMAC 验证密钥（敏感）
    /// - `api_version`: API 版本（非敏感）
    /// - `webhook_subscription_mode`: 订阅模式（非敏感）
    /// - `timeout`: HTTP 超时秒数（非敏感）
    ///
    /// Example shop domain configuration:
    /// ```json
    /// {
    ///   "config_type": "shopify",
    ///   "config_key": "shop_domain",
    ///   "config_value": "demo-store.myshopify.com",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example admin access token configuration:
    /// ```json
    /// {
    ///   "config_type": "shopify",
    ///   "config_key": "admin_access_token",
    ///   "config_value": "shpat_xxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ///   "is_secret": true,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example app client secret configuration:
    /// ```json
    /// {
    ///   "config_type": "shopify",
    ///   "config_key": "app_client_secret",
    ///   "config_value": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ///   "is_secret": true,
    ///   "enabled": true
    /// }
    /// ```
    Shopify,

    /// Email provider configuration
    ///
    /// Valid config_key values:
    /// - `provider`: Email provider type, either "resend" or "smtp" (non-secret)
    /// - `from_address`: Default sender email address (non-secret)
    /// - `resend_api_key`: Resend API key (secret, mark is_secret=true)
    /// - `smtp_host`: SMTP server hostname (non-secret)
    /// - `smtp_port`: SMTP server port number (non-secret, e.g., "587")
    /// - `smtp_username`: SMTP authentication username (non-secret)
    /// - `smtp_password`: SMTP authentication password (secret, mark is_secret=true)
    /// - `smtp_encryption`: Encryption mode, "starttls" or "ssl" (non-secret)
    ///
    /// Example provider configuration:
    /// ```json
    /// {
    ///   "config_type": "email",
    ///   "config_key": "provider",
    ///   "config_value": "resend",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example Resend API key configuration:
    /// ```json
    /// {
    ///   "config_type": "email",
    ///   "config_key": "resend_api_key",
    ///   "config_value": "re_your_api_key_here",
    ///   "is_secret": true,
    ///   "enabled": true
    /// }
    /// ```
    ///
    /// Example SMTP configuration:
    /// ```json
    /// {
    ///   "config_type": "email",
    ///   "config_key": "smtp_host",
    ///   "config_value": "smtp.example.com",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    Email,

    /// Invoice policy configuration
    ///
    /// Valid config_key values:
    /// - `policy`: Invoice policy settings as JSON string
    ///
    /// Example policy configuration:
    /// ```json
    /// {
    ///   "config_type": "invoice_policy",
    ///   "config_key": "policy",
    ///   "config_value": "{\"policy\":\"provider_first\",\"provider_capabilities\":{\"stripe\":{\"external_invoice_enabled\":true},\"creem\":{\"external_invoice_enabled\":true}}}",
    ///   "is_secret": false,
    ///   "enabled": true
    /// }
    /// ```
    InvoicePolicy,
}

impl From<String> for ConfigType {
    fn from(s: String) -> Self {
        ConfigType::try_from_str(&s).unwrap_or(ConfigType::Turnstile)
    }
}

impl ConfigType {
    pub fn try_from_str(s: &str) -> Result<Self, String> {
        let config_type = match s.to_lowercase().as_str() {
            "turnstile" => ConfigType::Turnstile,
            "registration" => ConfigType::Registration,
            "totp" => ConfigType::Totp,
            "totp_key" => ConfigType::TotpKey,
            "creem" => ConfigType::Creem,
            "stripe" => ConfigType::Stripe,
            "shopify" => ConfigType::Shopify,
            "email" => ConfigType::Email,
            "invoice_policy" => ConfigType::InvoicePolicy,
            _ => return Err(format!("Invalid config type: {}", s)),
        };
        Ok(config_type)
    }
}

impl From<ConfigType> for String {
    fn from(ct: ConfigType) -> Self {
        match ct {
            ConfigType::Turnstile => "turnstile".to_string(),
            ConfigType::Registration => "registration".to_string(),
            ConfigType::Totp => "totp".to_string(),
            ConfigType::TotpKey => "totp_key".to_string(),
            ConfigType::Creem => "creem".to_string(),
            ConfigType::Stripe => "stripe".to_string(),
            ConfigType::Shopify => "shopify".to_string(),
            ConfigType::Email => "email".to_string(),
            ConfigType::InvoicePolicy => "invoice_policy".to_string(),
        }
    }
}

impl AsRef<str> for ConfigType {
    fn as_ref(&self) -> &str {
        match self {
            ConfigType::Turnstile => "turnstile",
            ConfigType::Registration => "registration",
            ConfigType::Totp => "totp",
            ConfigType::TotpKey => "totp_key",
            ConfigType::Creem => "creem",
            ConfigType::Stripe => "stripe",
            ConfigType::Shopify => "shopify",
            ConfigType::Email => "email",
            ConfigType::InvoicePolicy => "invoice_policy",
        }
    }
}

/// 创建/更新配置请求
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct UpsertRealmConfigRequest {
    /// Configuration type (totp, turnstile, registration)
    ///
    /// See ConfigType documentation for details on each type
    #[schema(example = "totp")]
    #[serde(rename = "configType")]
    pub config_type: ConfigType,

    /// Configuration key (specific to each config_type)
    ///
    /// **TOTP**: `settings` (fixed key, stores JSON object with enabled/force_enabled)
    /// **Turnstile**: `site_key`, `secret_key`
    /// **Registration**: `allowed_domains`, `require_email_verification`
    #[schema(example = "settings")]
    #[serde(rename = "configKey")]
    pub config_key: String,

    /// Configuration value (format depends on config_key)
    ///
    /// For TOTP `settings`: JSON string like `{"enabled":true,"force_enabled":false}`
    /// For Turnstile keys: The actual key string
    /// For Registration `allowed_domains`: Comma-separated domains (e.g., "example.com,test.org")
    /// For Registration `require_email_verification`: "true" or "false"
    #[schema(example = r#"{"enabled":true,"force_enabled":false}"#)]
    #[serde(rename = "configValue")]
    pub config_value: String,

    /// Whether this value contains sensitive data (e.g., API keys)
    ///
    /// Set to true for secret keys, passwords, etc. Secret values may be masked in logs
    #[schema(example = "false")]
    #[serde(rename = "isSecret")]
    pub is_secret: Option<bool>,

    /// Whether this configuration is currently active
    ///
    /// Only applies to certain config types (e.g., TOTP uses this as the main enabled flag)
    #[schema(example = "true")]
    pub enabled: Option<bool>,

    /// Additional metadata (JSON object)
    ///
    /// Used for type-specific metadata:
    /// - TOTP: `{"force_enabled": boolean}` - whether TOTP is force-required
    /// - Other types: Can be used for future extensions
    #[schema(example = json!({"force_enabled": false}))]
    pub metadata: Option<serde_json::Value>,
}

/// API response type for realm configuration (with sensitive data hidden)
///
/// This type is used to return realm configuration data via API endpoints.
/// Sensitive data (where `is_secret=true`) is masked by setting `config_value` to `None`.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RealmConfigResponse {
    /// Configuration entry UUID
    pub id: Uuid,
    /// Realm this configuration belongs to
    #[serde(rename = "realmId")]
    pub realm_id: String,
    /// Configuration type (totp, turnstile, registration, totp_key)
    #[serde(rename = "configType")]
    pub config_type: ConfigType,
    /// Configuration key (specific to each config_type)
    #[serde(rename = "configKey")]
    pub config_key: String,
    /// Configuration value (hidden if is_secret=true)
    ///
    /// This field is `None` when `is_secret=true` to prevent leaking sensitive data.
    #[serde(rename = "configValue")]
    pub config_value: Option<String>,
    /// Whether this value contains sensitive data (e.g., API keys)
    #[serde(rename = "isSecret")]
    pub is_secret: bool,
    /// Whether this configuration is currently active
    pub enabled: bool,
    /// Additional metadata (JSON object)
    pub metadata: Option<serde_json::Value>,
    /// Creation timestamp
    #[serde(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl RealmConfig {
    /// Create a safe API response from RealmConfig
    ///
    /// This method creates a `RealmConfigResponse` that hides sensitive data.
    /// If `is_secret` is true, the `config_value` field is set to `None`.
    ///
    /// # Returns
    /// A `RealmConfigResponse` with sensitive data masked
    pub fn to_safe_response(self) -> RealmConfigResponse {
        RealmConfigResponse {
            id: self.id,
            realm_id: self.realm_id,
            config_type: self.config_type,
            config_key: self.config_key,
            config_value: if self.is_secret {
                None
            } else {
                Some(self.config_value)
            },
            is_secret: self.is_secret,
            enabled: self.enabled,
            metadata: self.metadata,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// 批量更新配置请求
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BatchUpsertRealmConfigRequest {
    /// List of configuration entries to create or update
    ///
    /// Each entry in the list follows the UpsertRealmConfigRequest structure
    #[schema(example = json!([
        {
            "config_type": "totp",
            "config_key": "settings",
            "config_value": "{\"enabled\":true,\"force_enabled\":false}",
            "is_secret": false,
            "enabled": true,
            "metadata": null
        },
        {
            "config_type": "turnstile",
            "config_key": "site_key",
            "config_value": "0x4AAAAAAxxxxxxxxxxxxxxxxxx",
            "is_secret": false,
            "enabled": true,
            "metadata": null
        }
    ]))]
    pub configs: Vec<UpsertRealmConfigRequest>,
}
