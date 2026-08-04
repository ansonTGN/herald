use std::fs;

#[derive(serde::Deserialize, Clone)]
pub struct ApiConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub server: ServerConfig,
    pub frontend: FrontendConfig,
    #[serde(default)]
    pub jwt: Option<JwtConfig>,
    #[serde(default)]
    _email: Option<EmailConfig>,
    /// Custom-domain global configuration (design §4.2.2 `cnameTarget`).
    ///
    /// `cname_target` is the Herald-owned hostname tenants must CNAME their
    /// custom login domain to; surfaced to realm admins in the GET response.
    /// `ask_key` is the shared secret for the Caddy On-Demand TLS ask
    /// authorization endpoint (validated/used by BE-D07).
    #[serde(default)]
    pub custom_domain: CustomDomainSettingsConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub google_oauth: GoogleOAuthConfig,
    #[serde(default)]
    pub apple_oauth: AppleOAuthConfig,
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_max_lifetime_secs")]
    pub max_lifetime_secs: u64,
    #[serde(default = "default_connect_timeout_secs")]
    pub connect_timeout_secs: u64,
}

fn default_max_connections() -> u32 {
    100
}

fn default_acquire_timeout_secs() -> u64 {
    30
}

fn default_idle_timeout_secs() -> u64 {
    600
}

fn default_max_lifetime_secs() -> u64 {
    1800
}

fn default_connect_timeout_secs() -> u64 {
    10
}

#[derive(serde::Deserialize, Clone)]
pub struct RedisConfig {
    #[serde(default = "default_redis_url")]
    pub url: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_app_env")]
    pub app_env: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct FrontendConfig {
    #[serde(default = "default_frontend_url")]
    pub url: String,
    /// 静态文件目录路径，用于 SPA 托管。None 则不托管静态文件
    #[serde(default)]
    pub static_dir: Option<String>,
}

#[derive(serde::Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct EmailConfig {
    _api_key: String,
}

/// Global custom-domain settings parsed from the `[custom_domain]` config
/// section (design §4.2.2 / §8 file-impact table).
#[derive(serde::Deserialize, Clone, Default)]
pub struct CustomDomainSettingsConfig {
    /// Herald-owned hostname tenants CNAME their custom login domain to
    /// (e.g. `custom.herald.com`). Surfaced to realm admins as `cnameTarget`
    /// in the GET response. Empty default keeps the field optional.
    #[serde(default)]
    pub cname_target: String,
    /// Shared secret for the Caddy On-Demand TLS ask authorization endpoint
    /// (design §4.2.2 ask). Validated and consumed by BE-D07; declared here
    /// so a single coordinated config section holds both keys.
    #[serde(default)]
    pub ask_key: String,
}

/// Google OAuth global settings parsed from the `[google_oauth]` config
/// section. Read from AppState (not the DB / per-realm config) so scenario
/// tests can override it on the test AppState to point at a wiremock JWKS.
#[derive(serde::Deserialize, Clone)]
pub struct GoogleOAuthConfig {
    #[serde(default = "default_google_jwks_url")]
    pub jwks_url: String,
}

fn default_google_jwks_url() -> String {
    herald_core::infrastructure::oauth::google::GoogleOAuthProvider::GOOGLE_JWKS_URL.to_string()
}

impl Default for GoogleOAuthConfig {
    fn default() -> Self {
        Self {
            jwks_url: default_google_jwks_url(),
        }
    }
}

/// Apple OAuth global settings parsed from the `[apple_oauth]` config section.
/// Same injection pattern as `GoogleOAuthConfig`: read from AppState (not the
/// DB / per-realm config) so scenario tests override it on the test AppState
/// to point at a wiremock JWKS.
#[derive(serde::Deserialize, Clone)]
pub struct AppleOAuthConfig {
    #[serde(default = "default_apple_jwks_url")]
    pub jwks_url: String,
}

fn default_apple_jwks_url() -> String {
    herald_core::infrastructure::oauth::apple::AppleOAuthProvider::JWKS_URL.to_string()
}

impl Default for AppleOAuthConfig {
    fn default() -> Self {
        Self {
            jwks_url: default_apple_jwks_url(),
        }
    }
}

#[derive(serde::Deserialize, Clone)]
pub struct ObservabilityConfig {
    #[serde(default = "default_service_name")]
    pub service_name: String,
    #[serde(default = "default_otlp_endpoint")]
    pub otlp_endpoint: String,
    #[serde(default = "default_metrics_export_interval_secs")]
    pub metrics_export_interval_secs: u64,
    #[serde(default)]
    pub traces_enabled: bool,
    #[serde(default = "default_sqlx_slow_statement_ms")]
    pub sqlx_slow_statement_ms: u64,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: default_service_name(),
            otlp_endpoint: default_otlp_endpoint(),
            metrics_export_interval_secs: default_metrics_export_interval_secs(),
            traces_enabled: false,
            sqlx_slow_statement_ms: default_sqlx_slow_statement_ms(),
        }
    }
}

fn default_service_name() -> String {
    "herald-api".to_string()
}

fn default_otlp_endpoint() -> String {
    "http://localhost:4318".to_string()
}

fn default_metrics_export_interval_secs() -> u64 {
    5
}

fn default_sqlx_slow_statement_ms() -> u64 {
    200
}

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_bind_address() -> String {
    "0.0.0.0:3000".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_app_env() -> String {
    "production".to_string()
}

fn default_frontend_url() -> String {
    "http://localhost:5173".to_string()
}

impl ApiConfig {
    pub fn load(path: &str) -> anyhow::Result<ApiConfig> {
        let config = fs::read_to_string(path)?;
        let cfg: ApiConfig = toml::from_str(&config)?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// User Story: Technical invariant — `ObservabilityConfig` defaults equal
    /// the design baseline (`.ai/design/observability.md`).
    /// Covers: "默认值 = baseline".
    ///
    /// WHY: the baseline is a security/cost contract — `traces_enabled=false`
    /// (no trace export by default), conservative OTLP endpoint, 5s metrics
    /// cadence, 200ms sqlx slow-statement threshold. If a default drifts, the
    /// baseline stops being the baseline and every downstream "traces off"
    /// assumption silently breaks. Lock all five fields to the documented
    /// values so a future change to the `Default` impl or a `#[serde(default)]`
    /// fn is caught here, not in production traffic.
    #[test]
    fn config_observability_defaults_match_baseline() {
        let cfg = ObservabilityConfig::default();
        assert_eq!(cfg.service_name, "herald-api");
        assert_eq!(cfg.otlp_endpoint, "http://localhost:4318");
        assert_eq!(cfg.metrics_export_interval_secs, 5);
        assert!(!cfg.traces_enabled, "baseline must default traces off");
        assert_eq!(cfg.sqlx_slow_statement_ms, 200);
    }

    /// User Story: Technical invariant — legacy configs without an
    /// `[observability]` section still parse and yield the baseline
    /// (`.ai/design/observability.md`, `#[serde(default)]` resilience).
    /// Covers: "缺 `[observability]` 段的旧 toml 仍解析".
    ///
    /// WHY: existing deployments ship `config.toml` files written before
    /// observability existed. A rollout of this code MUST NOT break them, and
    /// MUST NOT silently turn traces on either — the missing section must
    /// resolve to the exact baseline defaults, preserving default-off. This
    /// test fails if anyone removes `#[serde(default)]` from the `observability`
    /// field of `ApiConfig` or from the per-field defaults of
    /// `ObservabilityConfig`, either of which would make a pre-observability
    /// config fail to load or flip traces on.
    #[test]
    fn config_api_config_parses_without_observability_section() {
        // Minimal legal TOML for ApiConfig with NO `[observability]` section.
        // `database.url` is the only field without a serde default, so it must
        // be present; `[redis]`/`[server]`/`[frontend]` are required struct
        // fields but every one of their inner fields has a default, so the
        // empty section headers let them deserialize to their defaults.
        let toml = r#"
[database]
url = "postgres://test:test@localhost/test"

[redis]

[server]

[frontend]
"#;
        let cfg: ApiConfig = toml::from_str(toml).expect(
            "ApiConfig MUST parse a pre-observability config (no [observability] section) — \
             #[serde(default)] on the field is load-bearing for backwards compatibility",
        );

        // The missing section must resolve to the exact baseline, NOT to
        // something that turns traces on or points at a different collector.
        assert_eq!(cfg.observability.service_name, "herald-api");
        assert_eq!(cfg.observability.otlp_endpoint, "http://localhost:4318");
        assert_eq!(cfg.observability.metrics_export_interval_secs, 5);
        assert!(
            !cfg.observability.traces_enabled,
            "missing [observability] section MUST default to traces off (baseline isolation)"
        );
        assert_eq!(cfg.observability.sqlx_slow_statement_ms, 200);
    }
}
